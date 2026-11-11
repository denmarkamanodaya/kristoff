# roots/compute — outputs read by humans and the deploy workflow (none is a secret).

output "api_base_url" {
  description = "Base URL of the HTTP API (web/config.json apiBaseUrl)."
  value       = module.compute.api_base_url
}

output "stream_url" {
  description = "Function URL of ask-stream (web/config.json streamUrl)."
  value       = module.compute.stream_url
}

output "function_names" {
  description = "Names of every function, keyed by function key."
  value       = module.compute.function_names
}
