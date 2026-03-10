INSERT INTO permissions (role_id, resource, action)
SELECT r.id, 'roles', 'create'
FROM roles r
WHERE r.name = 'admin'
AND NOT EXISTS (
    SELECT 1 FROM permissions p
    WHERE p.role_id = r.id AND p.resource = 'roles' AND p.action = 'create'
);
