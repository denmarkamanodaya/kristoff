# s3-bucket v1 — outputs.

output "bucket_name" {
  description = "Bucket name."
  value       = aws_s3_bucket.this.bucket
}

output "bucket_arn" {
  description = "Bucket ARN (append /* for its objects)."
  value       = aws_s3_bucket.this.arn
}
