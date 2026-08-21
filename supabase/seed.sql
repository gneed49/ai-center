insert into app.workspaces (
  public_id,
  owner_actor_id,
  name
) values (
  '10000000-0000-0000-0000-000000000001',
  '00000000-0000-0000-0000-000000000001',
  'Mon workspace'
);

insert into app.project_templates (
  public_id,
  template_key,
  name,
  version,
  definition
) values (
  '20000000-0000-0000-0000-000000000001',
  'software-product-delivery',
  'Software Product Delivery',
  1,
  '{
    "root_nodes": ["product", "tech"],
    "handoffs": [{"from": "product", "to": "tech", "gate": "product-ready"}],
    "steward_profile": "project-steward"
  }'::jsonb
);

insert into app.agent_profiles (
  public_id,
  template_id,
  profile_key,
  name,
  scope_kind,
  instructions,
  retrieval_policy,
  output_schema,
  tools
)
select
  values_row.public_id,
  template.id,
  values_row.profile_key,
  values_row.name,
  values_row.scope_kind,
  values_row.instructions,
  values_row.retrieval_policy,
  values_row.output_schema,
  values_row.tools
from app.project_templates template
cross join (
  values
    (
      '21000000-0000-0000-0000-000000000001'::uuid,
      'product-agent',
      'Agent Produit',
      'product',
      'Challenge l intention, formalise objectif, règles métier, exigences, critères et questions. Propose des mutations atomiques sans jamais les confirmer à la place de l utilisateur.',
      '{"scope": "current_node", "neighbors": ["tech"], "include_project_summary": true}'::jsonb,
      '{"deliverable": "feature-brief", "proposal_types": ["business_rule", "requirement", "acceptance_criterion", "open_question"]}'::jsonb,
      '["propose_knowledge", "evaluate_product_gate", "generate_feature_brief"]'::jsonb
    ),
    (
      '21000000-0000-0000-0000-000000000002'::uuid,
      'tech-agent',
      'Agent Tech',
      'tech',
      'Transforme un ContextPack produit en plan de livraison technique sourcé. Explicite architecture, étapes, risques, tests, preuves et couverture sans reformuler le contexte.',
      '{"scope": "current_node", "context_pack_required": true, "include_linked_product_requirements": true}'::jsonb,
      '{"deliverable": "technical-delivery-plan", "proposal_types": ["technical_rule", "decision", "constraint"]}'::jsonb,
      '["propose_knowledge", "generate_technical_plan", "attach_evidence"]'::jsonb
    ),
    (
      '21000000-0000-0000-0000-000000000003'::uuid,
      'project-steward',
      'Steward du projet',
      'steward',
      'Analyse uniquement les connaissances confirmées et les projections. Détecte contradictions et trous de couverture avec sources, confiance, sévérité et action.',
      '{"scope": "project_condensed", "trigger": "domain_events", "candidate_filter": true}'::jsonb,
      '{"insight_types": ["contradiction", "coverage_gap"]}'::jsonb,
      '["detect_contradictions", "detect_coverage_gaps", "invalidate_projections"]'::jsonb
    )
) as values_row(
  public_id,
  profile_key,
  name,
  scope_kind,
  instructions,
  retrieval_policy,
  output_schema,
  tools
)
where template.template_key = 'software-product-delivery';

insert into app.deliverable_contracts (
  public_id,
  template_id,
  contract_key,
  name,
  required_sections,
  accepted_evidence_types,
  completion_rules,
  human_validation_required
)
select
  values_row.public_id,
  template.id,
  values_row.contract_key,
  values_row.name,
  values_row.required_sections,
  values_row.accepted_evidence_types,
  values_row.completion_rules,
  values_row.human_validation_required
from app.project_templates template
cross join (
  values
    (
      '22000000-0000-0000-0000-000000000001'::uuid,
      'feature-brief',
      'Feature Brief',
      array['objective', 'business_rules', 'requirements', 'acceptance_criteria', 'open_questions'],
      array['knowledge_entry', 'human_validation'],
      '{"gate": "product-ready", "all_sections_required": true}'::jsonb,
      true
    ),
    (
      '22000000-0000-0000-0000-000000000002'::uuid,
      'technical-delivery-plan',
      'Technical Delivery Plan',
      array['architecture', 'delivery_slices', 'risks', 'validation', 'coverage'],
      array['deliverable_section', 'analysis', 'human_validation'],
      '{"all_requirements_have_coverage_state": true}'::jsonb,
      true
    )
) as values_row(
  public_id,
  contract_key,
  name,
  required_sections,
  accepted_evidence_types,
  completion_rules,
  human_validation_required
)
where template.template_key = 'software-product-delivery';
