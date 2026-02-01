import {
	HeadContent,
	Outlet,
	Scripts,
	createRootRoute,
} from '@tanstack/react-router'

export const Route = createRootRoute({
	head: () => ({
		meta: [
			{
				charSet: 'utf-8',
			},
			{
				name: 'viewport',
				content: 'width=device-width, initial-scale=1',
			},
			{
				title: 'Duplex Stream',
			},
			{
				name: 'description',
				content: 'Turn AI conversations into lasting decisions',
			},
		],
		links: [
			{
				rel: 'icon',
				href: '/favicon.ico',
			},
		],
	}),
	component: RootComponent,
})

function RootComponent() {
	return (
		<html lang="en">
			<head>
				<HeadContent />
			</head>
			<body className="min-h-screen bg-background font-sans antialiased">
				<Outlet />
				<Scripts />
			</body>
		</html>
	)
}
