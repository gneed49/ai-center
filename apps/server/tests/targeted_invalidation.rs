//! Branch-shaped provenance fixtures exercise the runtime service under RLS.
//! Only fixture creation/inspection uses the separate isolated-database admin.
use std::sync::Arc;

use ai_center_server::{
    agent::DeterministicEngine,
    context::{ContextCandidate, compile_context_pack},
    error::AppError,
    models::{
        CompileContextPack, CreateHandoff, CreateProject, CreateSession, GenerateTechnicalPlan,
        InsightResolutionMutation, ResolveInsight, ReviseKnowledge,
    },
    service::{self, AppState},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

struct Fixture {
    state: AppState,
    admin: PgPool,
    project_id: i64,
    project_public_id: Uuid,
    session_public_id: Uuid,
    product_node_id: i64,
    tech_node_id: i64,
    tech_profile_id: i64,
    contract_id: i64,
}

struct Knowledge {
    id: i64,
    version_id: i64,
    candidate: ContextCandidate,
}

struct Projection {
    id: i64,
    public_id: Uuid,
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn resolution_invalidates_only_version_dependencies_and_traces_missing_provenance()
-> Result<()> {
    let fixture = fixture().await?;
    let source_a = knowledge(&fixture, "requirement", "Archiver les factures").await?;
    let source_b = knowledge(&fixture, "requirement", "Afficher les horaires").await?;
    knowledge(&fixture, "business_rule", "Chaque demande est confirmée").await?;
    knowledge(
        &fixture,
        "acceptance_criterion",
        "La confirmation reste visible",
    )
    .await?;
    let pack_a = pack(&fixture, "branch-a", Some(&source_a)).await?;
    let pack_b = pack(&fixture, "branch-b", Some(&source_b)).await?;
    // A candidate explicitly excluded from branch B is not a dependency.
    sqlx::query("insert into app.context_pack_selection_items(workspace_id,project_id,context_pack_id,candidate_kind,candidate_public_id,knowledge_entry_version_id,decision,reason_code) values($1,$2,$3,'knowledge_entry_version',$4,$5,'excluded','not_selected')")
        .bind(fixture.state.workspace_internal_id).bind(fixture.project_id).bind(pack_b.id)
        .bind(source_a.candidate.version_public_id).bind(source_a.version_id)
        .execute(&fixture.admin).await?;
    let legacy_pack = pack(&fixture, "legacy", None).await?;
    let deliverable_a = deliverable(&fixture, "branch-a", Some(&pack_a), None).await?;
    let deliverable_b = deliverable(&fixture, "branch-b", None, Some(&source_b)).await?;
    let legacy_deliverable = deliverable(&fixture, "legacy", None, None).await?;
    let direct_evidence_deliverable =
        deliverable(&fixture, "direct-evidence", None, Some(&source_b)).await?;
    let evidence_a = evidence(&fixture, &deliverable_a, &source_b).await?;
    let evidence_b = evidence(&fixture, &deliverable_b, &source_b).await?;
    let legacy_evidence = evidence(&fixture, &legacy_deliverable, &source_b).await?;
    let direct_evidence = evidence(&fixture, &direct_evidence_deliverable, &source_a).await?;
    let insight_public_id = insight(&fixture, &source_a).await?;
    let historical_pack_a = service::context_pack(&fixture.state, pack_a.public_id).await?;
    let historical_pack_b = service::context_pack(&fixture.state, pack_b.public_id).await?;
    let handoff_b = service::create_handoff(
        &fixture.state,
        fixture.project_public_id,
        CreateHandoff {
            source_session_id: fixture.session_public_id,
            context_pack_id: pack_b.public_id,
        },
    )
    .await?;

    let counters_before = counters(&fixture).await?;
    let unchanged = service::resolve_insight(
        &fixture.state,
        fixture.project_public_id,
        insight_public_id,
        resolution(&source_a, format!(" {} \n", source_a.candidate.statement)),
    )
    .await;
    ensure!(
        matches!(unchanged, Err(AppError::Invalid(_))),
        "an identical revision must be rejected"
    );
    ensure!(
        counters(&fixture).await? == counters_before,
        "no-op resolution mutated graph, versions, audit, or outbox"
    );
    ensure!(
        service::insight_detail(&fixture.state, insight_public_id)
            .await?
            .insight
            .status
            == "open"
    );
    let no_op_revision = service::revise_knowledge_for_project(
        &fixture.state,
        fixture.project_public_id,
        source_a.candidate.knowledge_public_id,
        ReviseKnowledge {
            statement: source_a.candidate.statement.clone(),
            title: None,
            rationale: None,
        },
    )
    .await;
    ensure!(matches!(no_op_revision, Err(AppError::Invalid(_))));
    ensure!(
        counters(&fixture).await? == counters_before,
        "no-op ordinary revision must roll back too"
    );

    let resolved = service::resolve_insight(
        &fixture.state,
        fixture.project_public_id,
        insight_public_id,
        resolution(
            &source_a,
            "Archiver les factures après confirmation humaine".into(),
        ),
    )
    .await?;
    ensure!(resolved.insight.status == "resolved");
    ensure!(resolved.insight.project_graph_version == 1);
    let current_a = service::context_pack(&fixture.state, pack_a.public_id).await?;
    let current_b = service::context_pack(&fixture.state, pack_b.public_id).await?;
    ensure!(current_a.status == "stale");
    ensure!(
        current_a.stale_reason.as_deref() == Some("included knowledge source version was revised")
    );
    ensure!(
        current_b.status == "current",
        "unrelated pack must not be invalidated"
    );
    ensure!(
        current_a.content == historical_pack_a.content
            && current_a.content_hash == historical_pack_a.content_hash
    );
    ensure!(
        current_b.content == historical_pack_b.content
            && current_b.content_hash == historical_pack_b.content_hash
    );
    ensure!(
        current_a.source_graph_version == 0 && current_b.source_graph_version == 0,
        "historical graph versions must not be rewritten"
    );
    let fallback = service::context_pack(&fixture.state, legacy_pack.public_id).await?;
    ensure!(
        fallback.status == "stale"
            && fallback
                .stale_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("missing source provenance"))
    );
    assert_deliverable(&fixture, &deliverable_a, "stale", "missing").await?;
    assert_deliverable(&fixture, &deliverable_b, "committed", "covered").await?;
    assert_deliverable(&fixture, &legacy_deliverable, "stale", "missing").await?;
    assert_deliverable(
        &fixture,
        &direct_evidence_deliverable,
        "committed",
        "missing",
    )
    .await?;
    for (id, expected) in [
        (evidence_a, "stale"),
        (evidence_b, "valid"),
        (legacy_evidence, "stale"),
        (direct_evidence, "stale"),
    ] {
        let actual: String = sqlx::query_scalar("select status from app.evidences where id = $1")
            .bind(id)
            .fetch_one(&fixture.admin)
            .await?;
        ensure!(
            actual == expected,
            "evidence {id}: expected {expected}, got {actual}"
        );
    }
    let audit: Value = sqlx::query_scalar("select after_state from app.audit_events where project_id = $1 and action = 'projections.invalidated'")
        .bind(fixture.project_id).fetch_one(&fixture.admin).await?;
    ensure!(audit["mode"] == "version_dependencies");
    ensure!(audit["source_version_ids"] == json!([source_a.candidate.version_public_id]));
    ensure!(audit["resulting_graph_version"] == 1);
    ensure!(audit["fallback"]["reason"] == "missing_source_provenance");
    ensure!(audit["fallback"]["context_pack_ids"] == json!([legacy_pack.public_id]));
    ensure!(audit["fallback"]["deliverable_ids"] == json!([legacy_deliverable.public_id]));
    ensure!(
        !audit["context_pack_ids"]
            .as_array()
            .context("audit pack ids")?
            .contains(&json!(pack_b.public_id))
    );

    // An unchanged historical branch still cannot authorize new work against
    // a different project graph. A fresh compilation is mandatory.
    let stale_handoff = service::create_handoff(
        &fixture.state,
        fixture.project_public_id,
        CreateHandoff {
            source_session_id: fixture.session_public_id,
            context_pack_id: pack_b.public_id,
        },
    )
    .await;
    ensure!(matches!(stale_handoff, Err(AppError::Conflict(_))));
    let stale_generation = service::generate_technical_plan(
        &fixture.state,
        fixture.project_public_id,
        GenerateTechnicalPlan {
            session_id: handoff_b.target_session_public_id,
        },
    )
    .await;
    ensure!(matches!(stale_generation, Err(AppError::Conflict(_))));
    let recompiled = service::compile_context_pack(
        &fixture.state,
        fixture.project_public_id,
        CompileContextPack {
            source_session_id: fixture.session_public_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: None,
        },
    )
    .await?;
    ensure!(recompiled.source_graph_version == 1 && recompiled.status == "current");
    let fresh_handoff = service::create_handoff(
        &fixture.state,
        fixture.project_public_id,
        CreateHandoff {
            source_session_id: fixture.session_public_id,
            context_pack_id: recompiled.public_id,
        },
    )
    .await?;
    let plan = service::generate_technical_plan(
        &fixture.state,
        fixture.project_public_id,
        GenerateTechnicalPlan {
            session_id: fresh_handoff.target_session_public_id,
        },
    )
    .await?;
    ensure!(plan.status == "committed" && plan.coverage_status == "missing");
    // The ordinary revision command shares the same exact dependency rules.
    let revision_b = service::revise_knowledge_for_project(
        &fixture.state,
        fixture.project_public_id,
        source_b.candidate.knowledge_public_id,
        ReviseKnowledge {
            statement: "Afficher les horaires confirmés".into(),
            title: None,
            rationale: None,
        },
    )
    .await?;
    ensure!(revision_b.graph_version == 2);
    ensure!(
        service::context_pack(&fixture.state, pack_b.public_id)
            .await?
            .status
            == "stale"
    );
    assert_deliverable(&fixture, &deliverable_b, "stale", "missing").await?;
    Ok(())
}

fn resolution(source: &Knowledge, statement: String) -> ResolveInsight {
    ResolveInsight {
        expected_graph_version: 0,
        justification: "La règle doit préciser la confirmation humaine".into(),
        mutations: vec![InsightResolutionMutation::ReviseKnowledge {
            knowledge_public_id: source.candidate.knowledge_public_id,
            expected_version_public_id: source.candidate.version_public_id,
            statement,
            title: None,
            rationale: None,
        }],
    }
}

async fn fixture() -> Result<Fixture> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must explicitly select an isolated test database")?;
    let admin_url = std::env::var("AI_CENTER_ADMIN_DATABASE_URL")
        .context("AI_CENTER_ADMIN_DATABASE_URL must select the same isolated database")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let admin = PgPoolOptions::new()
        .max_connections(3)
        .connect(&admin_url)
        .await?;
    if let Ok(expected_role) = std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE") {
        let actual: String = sqlx::query_scalar("select current_user")
            .fetch_one(&pool)
            .await?;
        ensure!(actual == expected_role);
    }
    let workspace_id = "10000000-0000-0000-0000-000000000001".parse()?;
    let actor_id = "00000000-0000-0000-0000-000000000001".parse()?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id, role from app.authorize_workspace_member($1, $2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(&pool)
            .await?;
    let state = AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role,
        actor_id,
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    let project = service::create_project(
        &state,
        CreateProject {
            name: format!("Targeted invalidation {}", Uuid::new_v4()),
            objective: "Conserver des preuves indépendantes et traçables".into(),
        },
    )
    .await?;
    let session = service::create_session(
        &state,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    let (project_id, product_node_id, tech_node_id, tech_profile_id, contract_id): (i64, i64, i64, i64, i64) = sqlx::query_as(
        "select p.id, product.id, tech.id, tech.agent_profile_id, contract.id
         from app.projects p
         join app.context_nodes product on product.project_id=p.id and product.node_key='product'
         join app.context_nodes tech on tech.project_id=p.id and tech.node_key='tech'
         join app.deliverable_contracts contract on contract.template_id=p.template_id and contract.contract_key='technical-delivery-plan'
         where p.public_id=$1",
    ).bind(project.public_id).fetch_one(&admin).await?;
    Ok(Fixture {
        state,
        admin,
        project_id,
        project_public_id: project.public_id,
        session_public_id: session.session.public_id,
        product_node_id,
        tech_node_id,
        tech_profile_id,
        contract_id,
    })
}

async fn knowledge(f: &Fixture, entry_type: &str, statement: &str) -> Result<Knowledge> {
    let (id, public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.knowledge_entries(workspace_id,project_id,context_node_id,entry_type)
         values($1,$2,$3,$4) returning id,public_id",
    )
    .bind(f.state.workspace_internal_id)
    .bind(f.project_id)
    .bind(f.product_node_id)
    .bind(entry_type)
    .fetch_one(&f.admin)
    .await?;
    let (version_id, version_public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.knowledge_entry_versions(knowledge_entry_id,workspace_id,project_id,context_node_id,version_number,entry_type,title,statement,author_actor_id,origin_type)
         values($1,$2,$3,$4,1,$5,$6,$6,$7,'test_fixture') returning id,public_id",
    ).bind(id).bind(f.state.workspace_internal_id).bind(f.project_id).bind(f.product_node_id).bind(entry_type).bind(statement).bind(f.state.actor_id).fetch_one(&f.admin).await?;
    Ok(Knowledge {
        id,
        version_id,
        candidate: ContextCandidate {
            knowledge_public_id: public_id,
            version_public_id,
            version_number: 1,
            entry_type: entry_type.into(),
            title: statement.into(),
            statement: statement.into(),
            rationale: String::new(),
            node_key: "product".into(),
        },
    })
}

async fn pack(f: &Fixture, kind: &str, source: Option<&Knowledge>) -> Result<Projection> {
    let candidates = source
        .map(|source| vec![source.candidate.clone()])
        .unwrap_or_default();
    let compiled = compile_context_pack(
        "Objectif de branche",
        "Résumé",
        0,
        &json!({}),
        &candidates,
        &[],
        12_000,
        "deterministic",
    )?;
    let (id, public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.context_packs(workspace_id,project_id,source_node_id,target_node_id,target_agent_profile_id,task_kind,objective,content,source_graph_version,compiler_version,content_hash)
         values($1,$2,$3,$4,$5,$6,'Objectif de branche',$7,0,'fixture-v1',$8) returning id,public_id",
    ).bind(f.state.workspace_internal_id).bind(f.project_id).bind(f.product_node_id).bind(f.tech_node_id).bind(f.tech_profile_id).bind(kind).bind(compiled.content).bind(compiled.content_hash).fetch_one(&f.admin).await?;
    if let Some(source) = source {
        sqlx::query("insert into app.context_pack_sources(workspace_id,project_id,context_pack_id,knowledge_entry_id,knowledge_entry_version_id,source_role,included_reason) values($1,$2,$3,$4,$5,'requirement','Branch fixture')")
            .bind(f.state.workspace_internal_id).bind(f.project_id).bind(id).bind(source.id).bind(source.version_id).execute(&f.admin).await?;
    }
    Ok(Projection { id, public_id })
}

async fn deliverable(
    f: &Fixture,
    name: &str,
    source_pack: Option<&Projection>,
    source_knowledge: Option<&Knowledge>,
) -> Result<Projection> {
    let (id, public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.deliverables(workspace_id,project_id,context_node_id,contract_id,source_graph_version,content_hash,deliverable_type,title,content,status,coverage_status,committed_at)
         values($1,$2,$3,$4,0,repeat('a',64),$5,$5,'{}','committed','covered',now()) returning id,public_id",
    ).bind(f.state.workspace_internal_id).bind(f.project_id).bind(f.tech_node_id).bind(f.contract_id).bind(name).fetch_one(&f.admin).await?;
    if let Some(source) = source_pack {
        sqlx::query("insert into app.deliverable_sources(workspace_id,project_id,deliverable_id,source_kind,source_public_id,context_pack_id,source_role) values($1,$2,$3,'context_pack',$4,$5,'compiled_context')")
            .bind(f.state.workspace_internal_id).bind(f.project_id).bind(id).bind(source.public_id).bind(source.id).execute(&f.admin).await?;
    }
    if let Some(source) = source_knowledge {
        sqlx::query("insert into app.deliverable_sources(workspace_id,project_id,deliverable_id,source_kind,source_public_id,knowledge_entry_version_id,source_role) values($1,$2,$3,'knowledge_entry_version',$4,$5,'requirement')")
            .bind(f.state.workspace_internal_id).bind(f.project_id).bind(id).bind(source.candidate.version_public_id).bind(source.version_id).execute(&f.admin).await?;
    }
    Ok(Projection { id, public_id })
}

async fn evidence(f: &Fixture, deliverable: &Projection, requirement: &Knowledge) -> Result<i64> {
    let section_id: i64 = sqlx::query_scalar("insert into app.deliverable_sections(workspace_id,project_id,deliverable_id,section_key,title,body,ordinal) values($1,$2,$3,'proof','Proof','Fixture',0) returning id")
        .bind(f.state.workspace_internal_id).bind(f.project_id).bind(deliverable.id).fetch_one(&f.admin).await?;
    let id: i64 = sqlx::query_scalar("insert into app.evidences(workspace_id,project_id,requirement_entry_id,requirement_version_id,deliverable_id,deliverable_section_id,evidence_type,title,source_reference,status) values($1,$2,$3,$4,$5,$6,'human_review','Validated fixture','test:fixture','valid') returning id")
        .bind(f.state.workspace_internal_id).bind(f.project_id).bind(requirement.id).bind(requirement.version_id).bind(deliverable.id).bind(section_id).fetch_one(&f.admin).await?;
    sqlx::query("insert into app.requirement_coverage(workspace_id,project_id,requirement_entry_id,requirement_version_id,deliverable_id,deliverable_section_id,evidence_id,status) values($1,$2,$3,$4,$5,$6,$7,'covered')")
        .bind(f.state.workspace_internal_id).bind(f.project_id).bind(requirement.id).bind(requirement.version_id).bind(deliverable.id).bind(section_id).bind(id).execute(&f.admin).await?;
    Ok(id)
}

async fn insight(f: &Fixture, source: &Knowledge) -> Result<Uuid> {
    let (id, public_id): (i64, Uuid) = sqlx::query_as("insert into app.insights(workspace_id,project_id,insight_type,status,severity,confidence,title,explanation) values($1,$2,'contradiction','open','warning',0.9,'Clarifier la confirmation','Le contexte doit préciser la confirmation') returning id,public_id")
        .bind(f.state.workspace_internal_id).bind(f.project_id).fetch_one(&f.admin).await?;
    sqlx::query("insert into app.insight_sources(workspace_id,project_id,insight_id,source_role,object_kind,object_public_id,knowledge_entry_version_id) values($1,$2,$3,'source','knowledge_entry_version',$4,$5)")
        .bind(f.state.workspace_internal_id).bind(f.project_id).bind(id).bind(source.candidate.version_public_id).bind(source.version_id).execute(&f.admin).await?;
    Ok(public_id)
}

async fn assert_deliverable(
    f: &Fixture,
    d: &Projection,
    expected_status: &str,
    expected_coverage: &str,
) -> Result<()> {
    let (status, coverage): (String, String) =
        sqlx::query_as("select status,coverage_status from app.deliverables where id=$1")
            .bind(d.id)
            .fetch_one(&f.admin)
            .await?;
    ensure!(
        status == expected_status && coverage == expected_coverage,
        "deliverable {}: got {status}/{coverage}, expected {expected_status}/{expected_coverage}",
        d.public_id
    );
    Ok(())
}

async fn counters(f: &Fixture) -> Result<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("select p.graph_version,(select count(*) from app.knowledge_entry_versions where project_id=p.id),(select count(*) from app.domain_events where project_id=p.id),(select count(*) from app.audit_events where project_id=p.id) from app.projects p where p.id=$1")
        .bind(f.project_id).fetch_one(&f.admin).await?)
}
