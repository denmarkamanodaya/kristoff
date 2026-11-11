# env/prod.compute.tfvars — compute-stack inputs of kristoff-prod. Non-secret values only. Contract-fixed values
# (MAX_ATTACHED_DOCS, MAX_HISTORY_TURNS, SESSION_TTL_SECONDS, sizes, timeouts, throttles) are stack defaults.

owner_id = "kristoff" # contract "Principal": the session token's sub and every storage key's owner

model_ids            = ["claude-sonnet-5-5", "claude-opus-5-5", "claude-haiku-5-5"] # first = default (contract "Model choice")
max_tokens           = 4096                                                         # one answer's output cap (backend env bound: <= 64000)
daily_question_cap   = 100                                                          # questions per UTC day before DAILY_CAP_REACHED (backend env bound: 1..10000)
anthropic_timeout_ms = 120000                                                       # one model call (backend env bound: <= 140000)
followup_model_id    = "claude-haiku-5-5"                                           # follow-up questions after each answer (contract v2 "Answer extras")
