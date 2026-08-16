#!/usr/bin/env node
// Local-only PR-12 identity fixture. It mirrors the production JWT verification
// contract (ES256 + kid + issuer + audience + email) without bypassing auth.
import http from 'node:http';
import crypto from 'node:crypto';

const PORT = Number(process.env.MOCK_IDP_PORT ?? 9100);
const ISSUER = `http://127.0.0.1:${PORT}/auth/v1`;
const KID = 'e2e-local-es256';
const FIXTURE_PASSWORD = 'e2e-password';

const { privateKey, publicKey } = crypto.generateKeyPairSync('ec', { namedCurve: 'P-256' });
const publicJwk = publicKey.export({ format: 'jwk' });

const USERS = new Map([
  ['e2e-admin@example.test', 'b0000000-0000-0000-0000-0000000000a0'],
  ['e2e-manager-a@example.test', 'b0000000-0000-0000-0000-0000000000a1'],
  ['e2e-manager-b@example.test', 'b0000000-0000-0000-0000-0000000000b1'],
  ['e2e-teacher-a@example.test', 'b0000000-0000-0000-0000-0000000000a2'],
  ['e2e-teacher-b@example.test', 'b0000000-0000-0000-0000-0000000000b2'],
  ['e2e-student-a@example.test', 'b0000000-0000-0000-0000-0000000000a3'],
  ['e2e-student-b@example.test', 'b0000000-0000-0000-0000-0000000000b3'],
  ['e2e-parent-a@example.test', 'b0000000-0000-0000-0000-0000000000a4'],
  ['e2e-parent-b@example.test', 'b0000000-0000-0000-0000-0000000000b4'],
  ['e2e-inactive@example.test', 'b0000000-0000-0000-0000-0000000000a9'],
]);

const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
function issueTokens(email, userId) {
  const now = Math.floor(Date.now() / 1000);
  const header = encode({ alg: 'ES256', typ: 'JWT', kid: KID });
  const claims = encode({
    sub: userId,
    email,
    aud: 'authenticated',
    iss: ISSUER,
    iat: now,
    exp: now + 900,
    role: 'authenticated',
  });
  const input = `${header}.${claims}`;
  const signature = crypto.sign('sha256', Buffer.from(input), {
    key: privateKey,
    dsaEncoding: 'ieee-p1363',
  }).toString('base64url');
  return {
    access_token: `${input}.${signature}`,
    token_type: 'bearer',
    expires_in: 900,
    refresh_token: `e2e-refresh-${userId}`,
    user: { id: userId, email, aud: 'authenticated' },
  };
}

function parseBody(body) {
  try {
    return JSON.parse(body || '{}');
  } catch {
    return null;
  }
}

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
  let body = '';
  req.on('data', (chunk) => { body += chunk; });
  req.on('end', () => {
    res.setHeader('content-type', 'application/json');

    if (url.pathname === '/auth/v1/token' && url.searchParams.get('grant_type') === 'password') {
      const payload = parseBody(body);
      if (!payload) {
        res.statusCode = 400;
        res.end(JSON.stringify({ error: 'invalid_request' }));
        return;
      }
      const userId = USERS.get(payload.email);
      if (userId && payload.password === FIXTURE_PASSWORD) {
        res.end(JSON.stringify(issueTokens(payload.email, userId)));
      } else {
        res.statusCode = 400;
        res.end(JSON.stringify({ error: 'invalid_grant', error_description: 'Invalid login credentials' }));
      }
      return;
    }

    if (url.pathname === '/auth/v1/token' && url.searchParams.get('grant_type') === 'refresh_token') {
      const payload = parseBody(body);
      const entry = payload && [...USERS.entries()].find(([, userId]) =>
        `e2e-refresh-${userId}` === payload.refresh_token,
      );
      if (entry) {
        res.end(JSON.stringify(issueTokens(entry[0], entry[1])));
      } else {
        res.statusCode = 400;
        res.end(JSON.stringify({ error: 'invalid_grant' }));
      }
      return;
    }

    if (url.pathname === '/auth/v1/.well-known/jwks.json') {
      res.end(JSON.stringify({
        keys: [{
          kty: 'EC', crv: 'P-256', alg: 'ES256', use: 'sig', kid: KID,
          x: publicJwk.x, y: publicJwk.y,
        }],
      }));
      return;
    }

    res.statusCode = 404;
    res.end(JSON.stringify({ error: 'not_found' }));
  });
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`mock-idp listening on ${ISSUER}`);
});
