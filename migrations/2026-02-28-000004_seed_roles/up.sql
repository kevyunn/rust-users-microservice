-- Insert base roles
INSERT INTO roles (name) VALUES
    ('admin'),
    ('manager'),
    ('user');

-- Admin permissions: full access to users and roles
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

-- Manager permissions: create and read users
INSERT INTO permissions (role_id, resource, action)
SELECT r.id, p.resource, p.action
FROM roles r
CROSS JOIN (
    VALUES
        ('users', 'create'),
        ('users', 'read')
) AS p(resource, action)
WHERE r.name = 'manager';

-- User permissions: read and update own profile
INSERT INTO permissions (role_id, resource, action)
SELECT r.id, p.resource, p.action
FROM roles r
CROSS JOIN (
    VALUES
        ('users', 'read_own'),
        ('users', 'update_own')
) AS p(resource, action)
WHERE r.name = 'user';
