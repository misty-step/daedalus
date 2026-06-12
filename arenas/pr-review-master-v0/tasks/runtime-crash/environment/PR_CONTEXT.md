# PR Context: runtime-crash

The PR removes defensive payload access from the ingest path. Normal webhook traffic can omit optional `payload` for ping and dry-run events. Security artifacts include one speculative field-injection concern that lacks an authorization path.
