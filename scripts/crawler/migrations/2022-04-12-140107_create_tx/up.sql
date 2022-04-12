CREATE TABLE tx (
  id SERIAL PRIMARY KEY,
  sig VARCHAR NOT NULL,
  input_token VARCHAR NOT NULL,
  output_token VARCHAR NOT NULL,
  input_amount BIGINT NOT NULL,
  output_amount BIGINT NOT NULL
);