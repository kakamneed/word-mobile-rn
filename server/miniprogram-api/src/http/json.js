export async function readJsonBody(request) {
  const chunks = [];
  for await (const chunk of request) {
    chunks.push(chunk);
  }
  const text = Buffer.concat(chunks).toString('utf8');
  if (!text) {
    return {};
  }
  return JSON.parse(text);
}

export function sendJson(response, statusCode, body) {
  response.writeHead(statusCode, {
    'Content-Type': 'application/json; charset=utf-8',
  });
  response.end(JSON.stringify(body));
}

export function sendError(response, error) {
  sendJson(response, error.statusCode ?? 500, {
    error: {
      code: error.code ?? 'REQUEST_FAILED',
      message: error.message ?? 'Request failed',
    },
  });
}
