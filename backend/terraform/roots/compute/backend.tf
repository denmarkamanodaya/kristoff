# roots/compute — partial S3 backend. The bucket, key, region and lockfile come ONLY from
# env/<stage>.compute.backend.hcl through tf.sh, which pairs it with env/<stage>.tfvars; the stage guard then compares the
# initialised backend with the tfvars stage and fails the plan on a mismatch.

terraform {
  backend "s3" {}
}
