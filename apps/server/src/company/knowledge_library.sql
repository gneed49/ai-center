with matching as (
 select v.id,v.public_id,v.title,left(v.statement,240) excerpt,v.entry_type,v.version_number,v.created_at,
  case when v.version_number=e.latest_version then e.status else 'superseded' end status,
  p.public_id project_public_id,p.name project_name,p.status project_status,p.scope_kind
 from app.knowledge_entry_versions v join app.knowledge_entries e on e.id=v.knowledge_entry_id
 join app.projects p on p.id=v.project_id
 where v.workspace_id=app.current_workspace_id() and ($1::uuid is null or p.public_id=$1)
  and ($2='' or strpos(lower(v.title),lower($2))>0 or strpos(lower(v.statement),lower($2))>0)
  and ($3 or v.version_number=e.latest_version)
), page as (select * from matching order by created_at desc,id desc limit $4 offset $5)
select jsonb_build_object('total',(select count(*) from matching),'limit',$4,'offset',$5,
 'items',coalesce((select jsonb_agg(jsonb_build_object(
  'public_id',public_id,'title',title,'excerpt',excerpt,'entry_type',entry_type,'version_number',version_number,
  'status',status,'recorded_at',created_at,'project_public_id',project_public_id,'project_name',project_name,
  'project_status',project_status,'scope_kind',scope_kind) order by created_at desc,id desc) from page),'[]'))
