-- The full-text indexes of names and comments that the search reads: on
-- Turso, tantivy on the n-grams of the text; on PostgreSQL, a GIN index of
-- the text's words (see `uc_storage::postgres_ddl`).

CREATE INDEX catalog_search ON catalog USING fts (name, comment) WITH (tokenizer = 'ngram');
CREATE INDEX schema_search ON schema USING fts (name, comment) WITH (tokenizer = 'ngram');
CREATE INDEX tbl_search ON tbl USING fts (name, comment) WITH (tokenizer = 'ngram');
CREATE INDEX volume_search ON volume USING fts (name, comment) WITH (tokenizer = 'ngram');
CREATE INDEX func_search ON func USING fts (name, comment) WITH (tokenizer = 'ngram');
CREATE INDEX registered_model_search ON registered_model USING fts (name, comment) WITH (tokenizer = 'ngram');
