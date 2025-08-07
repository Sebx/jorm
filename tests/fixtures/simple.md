# DAG: simple
**Schedule:** Every 10 minutes

## Tasks
- `extract_sales`
- `transform_data`
- `load_to_db`

## Dependencies
- `transform_data` runs after `extract_sales`
- `load_to_db` runs after `transform_data`
