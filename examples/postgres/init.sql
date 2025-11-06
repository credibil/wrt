-- Create a sample table in the 'postgres' database
CREATE TABLE IF NOT EXISTS mytable (
    feed_id INT PRIMARY KEY,
    agency_id VARCHAR(64) NOT NULL,
    agency_name VARCHAR(128) NOT NULL,
    agency_url VARCHAR(256) NOT NULL,
    agency_timezone VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
