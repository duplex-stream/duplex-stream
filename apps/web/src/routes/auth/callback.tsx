import { createFileRoute } from '@tanstack/react-router'
import { getAuthkit } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/callback')({
	server: {
		handlers: {
			GET: async ({ request }: { request: Request }) => {
				const url = new URL(request.url)
				const code = url.searchParams.get('code')
				const state = url.searchParams.get('state')

				if (!code) {
					console.error('Missing authorization code')
					return new Response(null, {
						status: 302,
						headers: { Location: '/auth/login' },
					})
				}

				try {
					const response = new Response()
					const authkit = await getAuthkit()
					const result = await authkit.handleCallback(request, response, {
						code,
						state: state ?? undefined,
					})

					const { authResponse } = result
					console.log('User authenticated:', authResponse.user.email)

					// Check if this is a desktop flow
					const isDesktopFlow = state?.includes('desktop=true')

					// Determine redirect URL
					let redirectUrl: string
					if (isDesktopFlow) {
						// Redirect to desktop app with access token
						redirectUrl = `duplex://auth/callback?token=${encodeURIComponent(authResponse.accessToken)}`
					} else {
						// Normal web flow - redirect to dashboard
						redirectUrl = new URL('/dashboard', url.origin).toString()
					}

					// Extract session headers (cookies) from the result
					const headers = new Headers()
					headers.set('Location', redirectUrl)

					// Copy session cookies from the response
					const setCookieHeader = response.headers.get('set-cookie')
					if (setCookieHeader) {
						headers.set('set-cookie', setCookieHeader)
					}

					// Also check if result has headers with cookies
					if (result.headers) {
						for (const [key, value] of Object.entries(result.headers)) {
							if (key.toLowerCase() === 'set-cookie') {
								headers.set('set-cookie', value as string)
							}
						}
					}

					return new Response(null, {
						status: 307,
						headers,
					})
				} catch (error) {
					console.error('OAuth callback failed:', error)
					return new Response(null, {
						status: 302,
						headers: { Location: '/auth/login' },
					})
				}
			},
		},
	},
	component: () => null,
})
