-- Drupal Rust Schema
-- Based on Drupal 4.7.0 core tables

-- Users table
CREATE TABLE IF NOT EXISTS users (
    uid INT UNSIGNED NOT NULL AUTO_INCREMENT,
    name VARCHAR(60) NOT NULL DEFAULT '',
    pass VARCHAR(255) NOT NULL DEFAULT '',
    mail VARCHAR(64) DEFAULT '',
    status TINYINT NOT NULL DEFAULT 0,
    created INT NOT NULL DEFAULT 0,
    login INT NOT NULL DEFAULT 0,
    PRIMARY KEY (uid),
    UNIQUE KEY name (name),
    KEY mail (mail)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Anonymous user (uid 0)
INSERT IGNORE INTO users (uid, name, status) VALUES (0, '', 0);

-- Sessions table (for tower-sessions)
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(128) NOT NULL,
    data BLOB NOT NULL,
    expiry_date BIGINT NOT NULL,
    PRIMARY KEY (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Roles table
CREATE TABLE IF NOT EXISTS role (
    rid INT UNSIGNED NOT NULL AUTO_INCREMENT,
    name VARCHAR(64) NOT NULL DEFAULT '',
    PRIMARY KEY (rid),
    UNIQUE KEY name (name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Default roles
INSERT IGNORE INTO role (rid, name) VALUES (1, 'anonymous user');
INSERT IGNORE INTO role (rid, name) VALUES (2, 'authenticated user');
INSERT IGNORE INTO role (rid, name) VALUES (3, 'administrator');

-- Users roles mapping
CREATE TABLE IF NOT EXISTS users_roles (
    uid INT UNSIGNED NOT NULL DEFAULT 0,
    rid INT UNSIGNED NOT NULL DEFAULT 0,
    PRIMARY KEY (uid, rid),
    KEY rid (rid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Permissions table
CREATE TABLE IF NOT EXISTS permission (
    rid INT UNSIGNED NOT NULL DEFAULT 0,
    perm TEXT,
    PRIMARY KEY (rid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Default permissions
INSERT IGNORE INTO permission (rid, perm) VALUES (1, 'access content');
INSERT IGNORE INTO permission (rid, perm) VALUES (2, 'access content, create page content');
INSERT IGNORE INTO permission (rid, perm) VALUES (3, 'access content, create page content, edit own page content, edit any page content, delete own page content, delete any page content, administer nodes, administer users');

-- Node table
CREATE TABLE IF NOT EXISTS node (
    nid INT UNSIGNED NOT NULL AUTO_INCREMENT,
    vid INT UNSIGNED NOT NULL DEFAULT 0,
    type VARCHAR(32) NOT NULL DEFAULT '',
    title VARCHAR(255) NOT NULL DEFAULT '',
    uid INT UNSIGNED NOT NULL DEFAULT 0,
    status INT NOT NULL DEFAULT 1,
    created INT NOT NULL DEFAULT 0,
    changed INT NOT NULL DEFAULT 0,
    promote INT NOT NULL DEFAULT 0,
    sticky INT NOT NULL DEFAULT 0,
    PRIMARY KEY (nid),
    KEY node_changed (changed),
    KEY node_created (created),
    KEY node_promote_status (promote, status),
    KEY node_status_type (status, type, nid),
    KEY node_type (type),
    KEY uid (uid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Node revisions table
CREATE TABLE IF NOT EXISTS node_revisions (
    vid INT UNSIGNED NOT NULL AUTO_INCREMENT,
    nid INT UNSIGNED NOT NULL DEFAULT 0,
    uid INT UNSIGNED NOT NULL DEFAULT 0,
    title VARCHAR(255) NOT NULL DEFAULT '',
    body LONGTEXT,
    teaser LONGTEXT,
    timestamp INT NOT NULL DEFAULT 0,
    format INT NOT NULL DEFAULT 0,
    PRIMARY KEY (vid),
    KEY nid (nid),
    KEY uid (uid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Variables table (key-value store for site configuration)
CREATE TABLE IF NOT EXISTS variable (
    name VARCHAR(128) NOT NULL DEFAULT '',
    value LONGTEXT,
    PRIMARY KEY (name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Node types table
CREATE TABLE IF NOT EXISTS node_type (
    type VARCHAR(32) NOT NULL,
    name VARCHAR(255) NOT NULL DEFAULT '',
    description MEDIUMTEXT,
    help MEDIUMTEXT,
    PRIMARY KEY (type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Default node types
INSERT IGNORE INTO node_type (type, name, description) VALUES ('page', 'Page', 'A page is a simple piece of content.');
INSERT IGNORE INTO node_type (type, name, description) VALUES ('story', 'Story', 'A story is an article or blog post.');
