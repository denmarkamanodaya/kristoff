# s3-bucket v1 — a private, versioned bucket. Block Public Access on, ACLs off, TLS-only policy, SSE-S3, lifecycle
# (abort multipart, noncurrent expiry, per-prefix expiry) and optional CORS for browser uploads and downloads through
# presigned URLs.
# Traps: no notifications, no access logging (cost); `prevent_destroy`
# plus the plan guard block delete/replace and any BPA/versioning weakening; on a versioned bucket an expiration only
# writes a delete marker, so the housekeeping rule also removes noncurrent versions and orphaned delete markers.

locals {
  # Known at plan time, so the TLS-only policy and the module tests never wait for the apply.
  bucket_arn = "arn:aws:s3:::${var.name}"
}

resource "aws_s3_bucket" "this" {
  bucket = var.name
  tags   = merge(var.tags, { Name = var.name })

  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_s3_bucket_ownership_controls" "this" {
  bucket = aws_s3_bucket.this.id

  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

resource "aws_s3_bucket_public_access_block" "this" {
  bucket                  = aws_s3_bucket.this.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "this" {
  bucket = aws_s3_bucket.this.id

  versioning_configuration {
    status = "Enabled"
  }
}

#trivy:ignore:AVD-AWS-0132 SSE-S3 (AES256) is the encryption docs/contract.md (Storage) fixes for the docs bucket; a customer-managed KMS key would cost per month and per request for a single-owner app
resource "aws_s3_bucket_server_side_encryption_configuration" "this" {
  bucket = aws_s3_bucket.this.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
    bucket_key_enabled = false
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "this" {
  bucket = aws_s3_bucket.this.id

  dynamic "rule" {
    for_each = var.expiring_prefixes
    content {
      id     = "expire-${trimsuffix(rule.key, "/")}"
      status = "Enabled"

      filter {
        prefix = rule.key
      }

      expiration {
        days = rule.value
      }
    }
  }

  rule {
    id     = "housekeeping"
    status = "Enabled"

    filter {
      prefix = ""
    }

    abort_incomplete_multipart_upload {
      days_after_initiation = var.abort_incomplete_multipart_days
    }

    noncurrent_version_expiration {
      noncurrent_days = var.noncurrent_version_expiration_days
    }
  }

  rule {
    id     = "delete-markers"
    status = "Enabled"

    filter {
      prefix = ""
    }

    expiration {
      expired_object_delete_marker = true
    }
  }

  # A non-current-version rule is only valid once versioning is on.
  depends_on = [aws_s3_bucket_versioning.this]
}

# Browsers PUT (presigned upload) and GET (presigned link) straight to the bucket; only the listed origins may.
resource "aws_s3_bucket_cors_configuration" "this" {
  count = length(var.cors_allowed_origins) > 0 ? 1 : 0

  bucket = aws_s3_bucket.this.id

  cors_rule {
    allowed_methods = var.cors_allowed_methods
    allowed_origins = var.cors_allowed_origins
    allowed_headers = ["content-type"]
    expose_headers  = ["ETag"]
    max_age_seconds = 3600
  }
}

data "aws_iam_policy_document" "tls_only" {
  statement {
    sid       = "DenyInsecureTransport"
    effect    = "Deny"
    actions   = ["s3:*"]
    resources = [local.bucket_arn, "${local.bucket_arn}/*"]

    principals {
      type        = "*"
      identifiers = ["*"]
    }

    condition {
      test     = "Bool"
      variable = "aws:SecureTransport"
      values   = ["false"]
    }
  }
}

resource "aws_s3_bucket_policy" "this" {
  bucket = aws_s3_bucket.this.id
  policy = data.aws_iam_policy_document.tls_only.json

  # Block Public Access first: the policy is a Deny only, but the ordering keeps the apply deterministic.
  depends_on = [aws_s3_bucket_public_access_block.this]
}
