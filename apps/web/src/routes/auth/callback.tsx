import { createFileRoute } from '@tanstack/react-router'
import { handleCallbackRoute } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/callback')({
	server: {
		handlers: {
			GET: handleCallbackRoute({
				returnPathname: '/dashboard',
				onSuccess: async ({ user }) => {
					console.log('User authenticated:', user.email)
				},
				onError: ({ error }) => {
					console.error('Authentication failed:', error)
					return new Response(null, {
						status: 302,
						headers: { Location: '/auth/login' },
					})
				},
			}),
		},
	},
	component: () => null,
})
