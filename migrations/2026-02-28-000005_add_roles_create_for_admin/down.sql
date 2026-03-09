DELETE FROM permissions
WHERE resource = 'roles' AND action = 'create'
AND role_id IN (SELECT id FROM roles WHERE name = 'admin');
