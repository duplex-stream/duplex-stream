import { createFileRoute, redirect } from '@tanstack/react-router'
import { getSignInUrl } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/login')({
	loader: async () => {
		const signInUrl = await getSignInUrl()
		throw redirect({ href: signInUrl })
	},
	component: () => null,
})
