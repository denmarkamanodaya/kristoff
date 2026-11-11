# dynamodb-table v1 — outputs.

output "table_name" {
  description = "Table name."
  value       = aws_dynamodb_table.this.name
}

output "table_arn" {
  description = "Table ARN (append /index/* for its indexes)."
  value       = aws_dynamodb_table.this.arn
}

output "index_names" {
  description = "Sorted names of the global secondary indexes."
  value       = sort([for g in var.global_secondary_indexes : g.name])
}
