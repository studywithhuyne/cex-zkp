<main>
  <h1>Matching Engine Gateway</h1>
  <p>Nginx is serving this SPA and proxying <code>/api</code> + <code>/ws</code> to the Rust backend.</p>
  <ul>
    <li><a href="/health" target="_blank" rel="noreferrer">GET /health</a></li>
    <li><a href="/api/orderbook" target="_blank" rel="noreferrer">GET /api/orderbook</a></li>
  </ul>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
    background: #0f172a;
    color: #e2e8f0;
  }

  main {
    max-width: 720px;
    margin: 48px auto;
    padding: 24px;
    background: #1e293b;
    border-radius: 12px;
  }

  h1 {
    margin-top: 0;
  }

  a {
    color: #38bdf8;
    text-decoration: none;
  }

  a:hover {
    text-decoration: underline;
  }
</style>
