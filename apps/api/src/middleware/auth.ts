import { createRemoteJWKSet, jwtVerify } from 'jose'
import { HTTPException } from 'hono/http-exception'

import type { Context, MiddlewareHandler, Next } from 'hono'
import type { HonoEnv } from '../context'

// Cache JWKS instance per client ID
const jwksCache = new Map<string, ReturnType<typeof createRemoteJWKSet>>()

function getJWKS(clientId: string) {
	let jwks = jwksCache.get(clientId)
	if (!jwks) {
		jwks = createRemoteJWKSet(
			new URL(`https://api.workos.com/sso/jwks/${clientId}`)
		)
		jwksCache.set(clientId, jwks)
	}
	return jwks
}

export interface AuthClaims {
	sub: string // User ID
	org_id?: string // Organization ID
	permissions?: string[] // User permissions
	email?: string
	[key: string]: unknown
}

/**
 * Auth middleware that validates WorkOS JWT tokens.
 * Extracts user ID, org ID, and permissions from token claims
 * and sets them on the Hono context.
 */
export function authMiddleware(): MiddlewareHandler<HonoEnv> {
	return async (c: Context<HonoEnv>, next: Next) => {
		const clientId = c.env.WORKOS_CLIENT_ID
		if (!clientId) {
			throw new HTTPException(500, { message: 'WORKOS_CLIENT_ID not configured' })
		}

		const authHeader = c.req.header('Authorization')
		if (!authHeader) {
			throw new HTTPException(401, { message: 'Missing Authorization header' })
		}

		if (!authHeader.startsWith('Bearer ')) {
			throw new HTTPException(401, { message: 'Invalid Authorization header format' })
		}

		const token = authHeader.slice(7)
		if (!token) {
			throw new HTTPException(401, { message: 'Missing token' })
		}

		try {
			const jwks = getJWKS(clientId)
			const { payload } = await jwtVerify(token, jwks, {
				// WorkOS tokens are issued by the WorkOS API
				issuer: 'https://api.workos.com',
			})

			const claims = payload as AuthClaims

			// Set auth context from JWT claims
			c.set('userId', claims.sub)
			c.set('orgId', claims.org_id || '')
			c.set('permissions', claims.permissions || [])

			await next()
		} catch (error) {
			// Log the error for debugging but don't expose details
			console.error('JWT verification failed:', error)
			throw new HTTPException(401, { message: 'Invalid or expired token' })
		}
	}
}

/**
 * Optional auth middleware - sets auth context if token is present,
 * but doesn't fail if missing. Useful for routes that work
 * both with and without authentication.
 */
export function optionalAuthMiddleware(): MiddlewareHandler<HonoEnv> {
	return async (c: Context<HonoEnv>, next: Next) => {
		const clientId = c.env.WORKOS_CLIENT_ID
		const authHeader = c.req.header('Authorization')

		if (!clientId || !authHeader || !authHeader.startsWith('Bearer ')) {
			// No auth - continue without setting user context
			await next()
			return
		}

		const token = authHeader.slice(7)
		if (!token) {
			await next()
			return
		}

		try {
			const jwks = getJWKS(clientId)
			const { payload } = await jwtVerify(token, jwks, {
				issuer: 'https://api.workos.com',
			})

			const claims = payload as AuthClaims
			c.set('userId', claims.sub)
			c.set('orgId', claims.org_id || '')
			c.set('permissions', claims.permissions || [])
		} catch {
			// Token invalid - continue without auth context
		}

		await next()
	}
}
