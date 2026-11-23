UPDATE item SET
  title = $1,
  changed = $2,
  fields = fields || $3::jsonb || jsonb_build_object('field_topics', (
    SELECT COALESCE(jsonb_agg(t ORDER BY t), '[]'::jsonb)
    FROM (
      SELECT jsonb_array_elements_text(COALESCE(item.fields->'field_topics', '[]'::jsonb))
      UNION
      SELECT jsonb_array_elements_text(COALESCE($3::jsonb->'field_topics', '[]'::jsonb))
    ) AS u(t)
  )),
  language = COALESCE(
    (SELECT id FROM language WHERE id = $3::jsonb->>'field_language'),
    (SELECT id FROM language WHERE is_default ORDER BY weight LIMIT 1),
    'en')
WHERE id = $4::uuid
