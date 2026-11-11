# env/prod.tfvars — stage inputs shared by roots/platform and roots/compute for kristoff-prod. Non-secret values only.
# tf.sh is the only reader that passes it to Terraform, always together with env/prod.<stack>.backend.hcl.
# The web origin is not set here: it is the Amplify default domain (platform output web_origin), read by roots/compute.

stage      = "prod"
account_id = "815882254506" # kristoff-prod (OU kristoff); tf.sh refuses unless the credentials are this account
region     = "ap-southeast-2"
