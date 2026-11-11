# Service control policies for OU `kristoff`

Three documents attach to the organizational unit `kristoff` (`ou-2con-ekrgjz2b`) in the management account; the
default `FullAWSAccess` allow stays on the root. Each file must stay under 5,120 bytes (`wc -c *.json`).

| File | Policy id | Purpose |
|---|---|---|
| `kristoff-guardrails.json` | `p-cqyosgio` | no leaving the organization, no root principal, no IAM users/keys/login profiles, no CloudTrail or GuardDuty changes, no account-BPA change (except the `AWSReservedSSO_KristoffAdmin_*` role), no Bedrock invocation logging |
| `kristoff-region-allowlist.json` | `p-73ijv8ub` | deny every request outside `ap-southeast-2` except global services |
| `kristoff-cost-guardrail.json` | `p-qla3ur6x` | deny the services a serverless-only account never needs (EC2, RDS, NAT, OpenSearch, provisioned throughput, ...) so a leaked credential cannot run them |

## Status (2026-11-12): DONE — created and attached by the lead

- [x] OU `kristoff` = `ou-2con-ekrgjz2b` (root `r-2con`).
- [x] Account `kristoff-prod` = `815882254506`, moved into the OU.
- [x] Permission set `KristoffAdmin` (`ps-8210cc9bcc638d0e`) assigned to the owner; CLI profile `kristoff-prod`
      (`sso_session = dk`; log in with `aws sso login --sso-session dk`).
- [x] The three policies above created and attached to the OU.
- [x] Access Analyzer `validate-policy` (type `SERVICE_CONTROL_POLICY`): 0 findings for the three files.
- [x] Probe: `iam:CreateUser` and a `us-east-1` call from `kristoff-prod` are denied by SCP.

Do not create or attach them again. The steps below are the record, and the way to change a policy later.

## Why the allow-list is `ap-southeast-2` only

- Everything Kristoff uses is regional and runs in Sydney: Lambda (including function URLs), API Gateway, DynamoDB,
  S3, SSM, SNS, CloudWatch and Amplify Hosting. The Anthropic API is called over the internet, not through Bedrock.
- Global services are exempted by `NotAction` (IAM, STS, Organizations, Budgets, Cost Explorer, support, health,
  account, Service Quotas, pricing), not by allowing `us-east-1`.
- The site is served only on the Amplify default domain (`*.amplifyapp.com`, owner ruling 2026-11-12): no custom
  domain, no certificate of our own. **A custom domain with a CloudFront distribution or an ACM certificate of our own
  would need `us-east-1` and therefore a change to this file.**

The files carry the two Access-Analyzer fixes recorded for dexfs (which these are modelled on): OpenSearch Service uses
the `es:` prefix (`opensearch:Create*` is not an action), and `aws:PrincipalArn` needs `ArnLike`, not `StringLike`.

## Attach (record; owner, management-account profile)

```bash
cd backend/terraform/bootstrap/scp
OU_ID=ou-2con-ekrgjz2b

for name in kristoff-guardrails kristoff-region-allowlist kristoff-cost-guardrail; do
  POLICY_ID=$(aws organizations create-policy --type SERVICE_CONTROL_POLICY --name "$name" \
    --description "kristoff OU guardrail: $name" --content "file://$name.json" \
    --query 'Policy.PolicySummary.Id' --output text)
  aws organizations attach-policy --policy-id "$POLICY_ID" --target-id "$OU_ID"
done

# verify: three kristoff policies listed on the OU
aws organizations list-policies-for-target --target-id "$OU_ID" --filter SERVICE_CONTROL_POLICY
# pass condition: the admin cannot create an IAM user
aws iam create-user --user-name probe --profile kristoff-prod   # expect AccessDenied (explicit deny in an SCP)
```

Updating a policy later: `aws organizations update-policy --policy-id <id> --content file://<name>.json` (ids above),
after `aws accessanalyzer validate-policy --policy-type SERVICE_CONTROL_POLICY --policy-document file://<name>.json`.
Detaching one is a deliberate owner action, never part of a deploy.
