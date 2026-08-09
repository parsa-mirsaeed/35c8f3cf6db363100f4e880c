#!/usr/bin/env node
// PR-12 local mock identity provider.
//
// Stands in for Supabase Auth (password grant + refresh) and JWKS so the
// production-like server authenticates without any live external service
// (plan §5.2 / §1.1). Listens on 127.0.0.1:9100 and only issues tokens for the
// synthetic seed users. Deterministic HS256 signing for the local fixture only.
import http from 'node:http';
import crypto from 'node:crypto';

const PORT = Number(process.env.MOCK_IDP_PORT ?? 9100);
const SECRET = process.env.MOCK_IDP_SECRET ?? 'e2e-local-only-secret';
const ISSUER = `http://127.0.0.1:${PORT}/auth/v1`;

// email -> { id, password }
const USERS = new Map([
  ['e2e-admin@example.test',   { id: 'b0000000-0000-0000-0000-0000000000a0', password: 'e2e-password' }],
  ['e2e-manager-a@example.test',{ id: 'b0000000-0000-0000-0000-0000000000a1', password: 'e2e-password' }],
  ['e2e-manager-b@example.test',{ id: 'b0000000-0000-0000-0000-0000000000b1', password: 'e2e-password' }],
  ['e2e-teacher-a@example.test',{ id: 'b0000000-0000-0000-0000-0000000000a2', password: 'e2e-password' }],
  ['e2e-teacher-b@example.test',{ id: 'b0000000-0000-0000-0000-0000000000b2', password: 'e2e-password' }],
  ['e2e-student-a@example.test',{ id: 'b0000000-0000-0000-0000-0000000000a3', password: 'e2e-password' }],
  ['e2e-student-b@example.test',{ id: 'b0000000-0000-0000-0000-0000000000b3', password: 'e2e-password' }],
  ['e2e-parent-a@example.test', { id: 'b0000000-0000-0000-0000-0000000000a4', password: 'e2e-password' }],
  ['e2e-parent-b@example.test', { id: 'b0000000-0000-0000-0000-0000000000b4', password: 'e2e-password' }],
  ['e2e-inactive@example.test', { id: 'b0000000-0000-0000-0000-0000000000a9', password: 'e2e-password' }],
]);

const b64 = (obj) => Buffer.from(JSON.stringify(obj)).toString('base64url');
const sign = (payload) =>
  `${b64({ alg: 'HS256', typ: 'JWT' })}.${b64(payload)}.` +
  crypto.createHmac('sha256', SECRET).update(`${b64({ alg: 'HS256', typ: 'JWT' })}.${b64(payload)}`).digest('base64url');

const issueTokens = (user) => {
  const now = Math.floor(Date.now() / 1000);
  const access = sign({ sub: user.id, aud: 'authenticated', iss: ISSUER, iat: now, exp: now + 900, role: 'authenticated' });
  return {
    access_token: access,
    token_type: 'bearer',
    expires_in: 900,
    refresh_token: `e2e-refresh-${user.id}`,
    user: { id: user.id, aud: 'authenticated' },
  };
};

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
  let body = '';
  req.on('data', (chunk) => (body += chunk));
  req.on('end', () => {
    res.setHeader('content-type', 'application/json');

    if (url.pathname === '/auth/v1/token' && url.searchParams.get('grant_type') === 'password') {
      const { email, password } = JSON.parse(body || '{}');
      const user = USERS.get(email);
      if (user && user.password === password) {
        res.end(JSON.stringify(issueTokens(user)));
      } else {
        res.statusCode = 400;
        res.end(JSON.stringify({ error: 'invalid_grant', error_description: 'Invalid login credentials' }));
      }
      return;
    }

    if (url.pathname === '/auth/v1/token' && url.searchParams.get('grant_type') === 'refresh_token') {
      const { refresh_token } = JSON.parse(body || '{}');
      const entry = [...USERS.values()].find((u) => `e2e-refresh-${u.id}` === refresh_token);
      if (entry) {
        res.end(JSON.stringify(issueTokens(entry)));
      } else {
        res.statusCode = 400;
        res.end(JSON.stringify({ error: 'invalid_grant' }));
      }
      return;
    }

    if (url.pathname === '/auth/v1/.well-known/jwks.json') {
      const key = crypto.createSecretKey(Buffer.from(SECRET)).export({ format: 'jwk' });
      res.end(JSON.stringify({ keys: [{ kty: 'oct', alg: 'HS256', use: 'sig', kid: 'e2e-local', k: key.k }] }));
      return;
    }

    res.statusCode = 404;
    res.end(JSON.stringify({ error: 'not_found' }));
  });
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`mock-idp listening on ${ISSUER}`);
});
