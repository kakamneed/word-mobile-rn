import { createMiniprogramApiServer } from './server.js';

const port = Number(process.env.PORT ?? 8787);
const server = createMiniprogramApiServer();

server.listen(port, () => {
  console.log(`Mini Program API listening on http://127.0.0.1:${port}`);
});
