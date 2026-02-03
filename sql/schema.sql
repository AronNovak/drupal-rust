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
    theme VARCHAR(255) DEFAULT '',
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
INSERT IGNORE INTO permission (rid, perm) VALUES (1, 'access content, access comments');
INSERT IGNORE INTO permission (rid, perm) VALUES (2, 'access content, access comments, post comments, create page content');
INSERT IGNORE INTO permission (rid, perm) VALUES (3, 'access content, access comments, post comments, administer comments, create page content, edit own page content, edit any page content, delete own page content, delete any page content, administer nodes, administer users');

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
    comment INT NOT NULL DEFAULT 2,
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

-- Profile fields table (based on Drupal 4.7 profile module)
CREATE TABLE IF NOT EXISTS profile_fields (
    fid INT UNSIGNED NOT NULL AUTO_INCREMENT,
    title VARCHAR(255) DEFAULT NULL,
    name VARCHAR(128) NOT NULL DEFAULT '',
    explanation TEXT,
    category VARCHAR(255) DEFAULT NULL,
    page VARCHAR(255) DEFAULT NULL,
    type VARCHAR(128) DEFAULT NULL,
    weight TINYINT NOT NULL DEFAULT 0,
    required TINYINT NOT NULL DEFAULT 0,
    register TINYINT NOT NULL DEFAULT 0,
    visibility TINYINT NOT NULL DEFAULT 0,
    options TEXT,
    PRIMARY KEY (fid),
    UNIQUE KEY name (name),
    KEY category (category)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Profile values table
CREATE TABLE IF NOT EXISTS profile_values (
    fid INT UNSIGNED NOT NULL DEFAULT 0,
    uid INT UNSIGNED NOT NULL DEFAULT 0,
    value TEXT,
    PRIMARY KEY (fid, uid),
    KEY uid (uid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Node fields table (field definitions, like CCK/Field API)
CREATE TABLE IF NOT EXISTS node_field (
    field_name VARCHAR(32) NOT NULL,
    field_type VARCHAR(32) NOT NULL DEFAULT 'text',
    cardinality INT NOT NULL DEFAULT 1,
    settings TEXT,
    PRIMARY KEY (field_name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Node field instances (links fields to node types)
CREATE TABLE IF NOT EXISTS node_field_instance (
    id INT UNSIGNED NOT NULL AUTO_INCREMENT,
    field_name VARCHAR(32) NOT NULL,
    node_type VARCHAR(32) NOT NULL,
    label VARCHAR(255) NOT NULL DEFAULT '',
    description TEXT,
    required TINYINT NOT NULL DEFAULT 0,
    weight INT NOT NULL DEFAULT 0,
    widget_type VARCHAR(32) DEFAULT 'textfield',
    widget_settings TEXT,
    display_settings TEXT,
    PRIMARY KEY (id),
    UNIQUE KEY field_node_type (field_name, node_type),
    KEY node_type (node_type),
    KEY field_name (field_name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Node field data (stores field values per node revision)
CREATE TABLE IF NOT EXISTS node_field_data (
    id INT UNSIGNED NOT NULL AUTO_INCREMENT,
    nid INT UNSIGNED NOT NULL,
    vid INT UNSIGNED NOT NULL,
    field_name VARCHAR(32) NOT NULL,
    delta INT UNSIGNED NOT NULL DEFAULT 0,
    value_text TEXT,
    value_int BIGINT,
    value_float DOUBLE,
    PRIMARY KEY (id),
    UNIQUE KEY field_revision_delta (vid, field_name, delta),
    KEY nid (nid),
    KEY vid (vid),
    KEY field_name (field_name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- System table (modules and themes)
CREATE TABLE IF NOT EXISTS system (
    filename VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL DEFAULT '',
    type VARCHAR(12) NOT NULL DEFAULT '',
    description VARCHAR(255) DEFAULT '',
    status INT NOT NULL DEFAULT 0,
    throttle TINYINT NOT NULL DEFAULT 0,
    bootstrap INT NOT NULL DEFAULT 0,
    schema_version SMALLINT NOT NULL DEFAULT -1,
    weight INT NOT NULL DEFAULT 0,
    PRIMARY KEY (filename),
    KEY system_weight (weight),
    KEY system_type_name (type, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Default modules (required core modules)
INSERT IGNORE INTO system (filename, name, type, description, status, weight) VALUES
('modules/system', 'system', 'module', 'Handles general site configuration.', 1, 0),
('modules/node', 'node', 'module', 'Allows content to be submitted to the site.', 1, 0),
('modules/user', 'user', 'module', 'Manages user registration and login.', 1, 0),
('modules/filter', 'filter', 'module', 'Handles the filtering of content.', 1, 0),
('modules/block', 'block', 'module', 'Controls the boxes that are displayed around content.', 1, 0);

-- Optional modules (disabled by default)
INSERT IGNORE INTO system (filename, name, type, description, status, weight) VALUES
('modules/statistics', 'statistics', 'module', 'Logs access statistics for your site.', 0, 0);

-- Default themes
INSERT IGNORE INTO system (filename, name, type, description, status, weight) VALUES
('themes/bluemarine', 'bluemarine', 'theme', 'The default Drupal theme.', 1, 0),
('themes/pushbutton', 'pushbutton', 'theme', 'A modern, button-styled theme.', 1, 0);

-- Set default theme
INSERT IGNORE INTO variable (name, value) VALUES ('theme_default', 'bluemarine');

-- Access log table (statistics module)
CREATE TABLE IF NOT EXISTS accesslog (
    aid INT UNSIGNED NOT NULL AUTO_INCREMENT,
    sid VARCHAR(64) NOT NULL DEFAULT '',
    title VARCHAR(255) DEFAULT NULL,
    path VARCHAR(255) DEFAULT NULL,
    url VARCHAR(255) DEFAULT NULL,
    hostname VARCHAR(128) DEFAULT NULL,
    uid INT UNSIGNED NOT NULL DEFAULT 0,
    timer INT UNSIGNED NOT NULL DEFAULT 0,
    timestamp INT UNSIGNED NOT NULL DEFAULT 0,
    PRIMARY KEY (aid),
    KEY accesslog_timestamp (timestamp),
    KEY accesslog_uid (uid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Node counter table (statistics module)
CREATE TABLE IF NOT EXISTS node_counter (
    nid INT UNSIGNED NOT NULL,
    totalcount BIGINT UNSIGNED NOT NULL DEFAULT 0,
    daycount MEDIUMINT UNSIGNED NOT NULL DEFAULT 0,
    timestamp INT UNSIGNED NOT NULL DEFAULT 0,
    PRIMARY KEY (nid),
    KEY node_counter_totalcount (totalcount),
    KEY node_counter_daycount (daycount),
    KEY node_counter_timestamp (timestamp)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Statistics settings
INSERT IGNORE INTO variable (name, value) VALUES ('statistics_enable_access_log', '0');
INSERT IGNORE INTO variable (name, value) VALUES ('statistics_count_content_views', '0');
INSERT IGNORE INTO variable (name, value) VALUES ('statistics_flush_accesslog_timer', '259200');

-- Comments table (Drupal 4.7 comment module)
CREATE TABLE IF NOT EXISTS comments (
    cid INT UNSIGNED NOT NULL AUTO_INCREMENT,
    pid INT UNSIGNED NOT NULL DEFAULT 0,
    nid INT UNSIGNED NOT NULL DEFAULT 0,
    uid INT UNSIGNED NOT NULL DEFAULT 0,
    subject VARCHAR(64) NOT NULL DEFAULT '',
    comment LONGTEXT NOT NULL,
    hostname VARCHAR(128) NOT NULL DEFAULT '',
    timestamp INT NOT NULL DEFAULT 0,
    status TINYINT UNSIGNED NOT NULL DEFAULT 0,
    thread VARCHAR(255) NOT NULL DEFAULT '',
    name VARCHAR(60) DEFAULT NULL,
    mail VARCHAR(64) DEFAULT NULL,
    homepage VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY (cid),
    KEY nid (nid),
    KEY pid (pid),
    KEY timestamp (timestamp),
    KEY status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Node comment statistics table
CREATE TABLE IF NOT EXISTS node_comment_statistics (
    nid INT UNSIGNED NOT NULL,
    last_comment_timestamp INT NOT NULL DEFAULT 0,
    last_comment_name VARCHAR(60) DEFAULT NULL,
    last_comment_uid INT UNSIGNED NOT NULL DEFAULT 0,
    comment_count INT UNSIGNED NOT NULL DEFAULT 0,
    PRIMARY KEY (nid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Add comment module to system
INSERT IGNORE INTO system (filename, name, type, description, status, weight) VALUES
('modules/comment', 'comment', 'module', 'Allows users to comment on and discuss published content.', 1, 0);
