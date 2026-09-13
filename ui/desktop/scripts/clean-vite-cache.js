const fs = require('fs');
const path = require('path');

const desktopRoot = path.resolve(__dirname, '..');

const pathsToRemove = [
  path.join(desktopRoot, 'node_modules', '.vite'),
  path.join(desktopRoot, 'node_modules', '.vite-temp'),
  path.join(desktopRoot, '.vite'),
];

for (const targetPath of pathsToRemove) {
  if (!fs.existsSync(targetPath)) {
    continue;
  }

  try {
    fs.rmSync(targetPath, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 });
    console.log(`Removed ${path.relative(desktopRoot, targetPath)}`);
  } catch (err) {
    if (err.code === 'ENOTEMPTY' || err.code === 'EBUSY' || err.code === 'EPERM') {
      console.warn(
        `Warning: Could not remove ${path.relative(desktopRoot, targetPath)} (process may be active): ${err.message}`
      );
    } else {
      throw err;
    }
  }
}
