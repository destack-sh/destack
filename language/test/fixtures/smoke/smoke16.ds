const server = Bun.serve({
	// `routes` requires Bun v1.2.3+
	routes: {
		// Static routes
		'/api/status': new Response('OK'),

		// Dynamic routes
		'/users/:id': req => {
			return new Response(`Hello User ${req.params.id}!`);
		},

		// Per-HTTP method handlers
		'/api/posts': {
			GET: () => new Response('List posts'),
			POST: async req => {
				const body = await req.json();
				return Response.json({created: true, ...body});
			},
		},

		// Wildcard route for all routes that start with "/api/" and aren't otherwise matched
		'/api/*': Response.json({message: 'Not found'}, {status: 404}),

		// Redirect from /blog/hello to /blog/hello/world
		'/blog/hello': Response.redirect('/blog/hello/world'),

		// Serve a file by buffering it in memory
		'/favicon.ico': new Response(await Bun.file('./favicon.ico').bytes(), {
			headers: {
				'Content-Type': 'image/x-icon',
			},
		}),
	},

	// (optional) fallback for unmatched routes:
	// Required if Bun's version < 1.2.3
	fetch(req) {
		return new Response('Not Found', {status: 404});
	},
});

console.log(`Server running at ${server.url}`);

Bun.serve({
	routes: {
		'/login': req => {
			const cookies = req.cookies;

			// Set a cookie with various options
			cookies.set('user_id', '12345', {
				maxAge: 60 * 60 * 24 * 7, // 1 week
				httpOnly: true,
				secure: true,
				path: '/',
			});

			// Add a theme preference cookie
			cookies.set('theme', 'dark');

			// Modified cookies from the request are automatically applied to the response
			return new Response('Login successful');
		},
	},
});

import {test, expect} from 'bun:test';

let sharedState = 0;

// These tests must run in order
test.serial('first serial test', () => {
	sharedState = 1;
	expect(sharedState).toBe(1);
});

test.serial('second serial test', () => {
	// Depends on the previous test
	expect(sharedState).toBe(1);
	sharedState = 2;
});

// This test can run concurrently if --concurrent is enabled
test('independent test', () => {
	expect(true).toBe(true);
});

// Chaining test qualifiers
test.failing.each([1, 2, 3])('chained qualifiers %d', input => {
	expect(input).toBe(0); // This test is expected to fail for each input
});