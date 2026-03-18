INSERT INTO roles (name) VALUES
    ('admin'),
    ('manager'),
    ('user');

INSERT INTO permissions (role_id, resource, action)
SELECT r.id, p.resource, p.action
FROM roles r
CROSS JOIN (
    VALUES
        ('users', 'create'),
        ('users', 'read'),
        ('users', 'update'),
        ('users', 'delete'),
        ('roles', 'read'),
        ('roles', 'update')
) AS p(resource, action)
WHERE r.name = 'admin';

INSERT INTO permissions (role_id, resource, action)
SELECT r.id, p.resource, p.action
FROM roles r
CROSS JOIN (
    VALUES
        ('users', 'create'),
        ('users', 'read')
) AS p(resource, action)
WHERE r.name = 'manager';

INSERT INTO permissions (role_id, resource, action)
SELECT r.id, p.resource, p.action
FROM roles r
CROSS JOIN (
    VALUES
        ('users', 'read_own'),
        ('users', 'update_own')
) AS p(resource, action)
WHERE r.name = 'user';
