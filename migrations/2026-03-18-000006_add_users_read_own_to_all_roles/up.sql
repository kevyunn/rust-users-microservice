INSERT INTO permissions (role_id, resource, action)
SELECT r.id, 'users', 'read_own'
FROM roles r
WHERE r.name IN ('admin', 'manager', 'user')
ON CONFLICT (role_id, resource, action) DO NOTHING;
