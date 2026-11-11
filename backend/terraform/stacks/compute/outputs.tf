# compute stack — outputs read by humans and the deploy workflow (web/config.json comes from api_base_url + stream_url).

output "api_base_url" {
  description = "Base URL of the HTTP API (apiBaseUrl of web/config.json)."
  value       = module.api.api_endpoint
}

output "stream_url" {
  description = "Function URL of ask-stream without the trailing slash (streamUrl of web/config.json)."
  value       = trimsuffix(module.ask.function_url, "/")
}

output "function_names" {
  description = "Names of every function, keyed by function key."
  value       = merge({ for k, f in module.fn : k => f.function_name }, { (local.stream_function) = module.ask.function_name })
}

output "granted_actions" {
  description = "IAM actions of each function's role, keyed by function key (plan-time review of least privilege)."
  value       = merge({ for k, f in module.fn : k => f.granted_actions }, { (local.stream_function) = module.ask.granted_actions })
}
