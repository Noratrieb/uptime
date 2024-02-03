CREATE TABLE checks_series (
    request_time_range_start TIMESTAMP NOT NULL,
    request_time_range_end TIMESTAMP NOT NULL,
    website VARCHAR NOT NULL,
    result VARCHAR NOT NULL
)
