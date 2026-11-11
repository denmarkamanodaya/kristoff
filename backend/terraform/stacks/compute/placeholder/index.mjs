// Placeholder bundle: deployed until backend/dist/<function key>.zip exists. Answers 503 so a route that is wired
// but not built yet fails loudly instead of looking healthy.
export const handler = async () => ({
  statusCode: 503,
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ error: { code: "NOT_DEPLOYED" } }),
});
