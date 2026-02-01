import { createFileRoute, redirect } from '@tanstack/react-router'
import { signOut } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/logout')({
	loader: async () => {
		// Clear the session and redirect to home
		await signOut()
		throw redirect({ to: '/' })
	},
	component: () => null,
})
