# env/prod.compute.tfvars — compute-stack inputs of kristoff-prod. Non-secret values only. Contract-fixed values
# (MAX_ATTACHED_DOCS, MAX_HISTORY_TURNS, SESSION_TTL_SECONDS, sizes, timeouts, throttles) are stack defaults.

owner_id = "kristoff" # contract "Principal": the session token's sub and every storage key's owner

model_id             = "claude-sonnet-5-5" # plan: default model; Anthropic model ids at docs.claude.com
max_tokens           = 4096                # one answer's output cap (backend env bound: <= 64000)
daily_question_cap   = 100                 # questions per UTC day before DAILY_CAP_REACHED (backend env bound: 1..10000)
anthropic_timeout_ms = 120000              # one model call (backend env bound: <= 140000)
