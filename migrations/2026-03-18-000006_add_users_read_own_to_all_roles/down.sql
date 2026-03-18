DELETE FROM permissions p
USING roles r
WHERE p.role_id = r.id
  AND p.resource = 'users'
  AND p.action = 'read_own'
  AND r.name IN ('admin', 'manager', 'user');
