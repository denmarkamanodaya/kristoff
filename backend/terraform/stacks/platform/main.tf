# platform stack — the long-lived, data-bearing resources of Kristoff's one stage: the table, the docs bucket, the AI kill
# switch parameter, the alerts topic and the Amplify web app. Composition root of the data layer: it only wires modules
# (stack/terraform.md). The compute stack finds all of these by NAME (stage-names), never through remote state.
# Names and shapes: docs/contract.md ("Storage", "Config").

module "names" {
  source = "../../modules/stage-names/v1"

  stage      = var.stage
  account_id = var.account_id
  region     = var.region
}

# Docs, chats, messages, usage counters and TOTP state (contract "Storage"). Key layout is the backend's
# (backend/src/domain/keys); GSI1 serves "list chats by updatedAt". No TTL: the contract stores nothing that expires.
# The projection of GSI1 is immutable once created.
module "table" {
  source = "../../modules/dynamodb-table/v1"

  name          = module.names.table_name
  hash_key      = "PK"
  range_key     = "SK"
  ttl_attribute = null
  attribute_types = {
    PK     = "S"
    SK     = "S"
    GSI1PK = "S"
    GSI1SK = "S"
  }
  global_secondary_indexes = [
    {
      name            = "GSI1"
      hash_key        = "GSI1PK"
      range_key       = "GSI1SK"
      projection_type = "ALL"
    },
  ]
}

# The owner's PDFs, docs/<uid>/<docId>.pdf. Nothing expires (contract); an overwritten or deleted version stays
# recoverable for 90 days. The browser uploads (presigned PUT, Content-Type) and opens (presigned GET) straight from the
# one web origin, the Amplify branch URL (web.tf).
module "docs_bucket" {
  source = "../../modules/s3-bucket/v1"

  name                               = module.names.docs_bucket_name
  noncurrent_version_expiration_days = 90
  cors_allowed_origins               = [module.web.branch_url]
  cors_allowed_methods               = ["GET", "PUT"]
}
