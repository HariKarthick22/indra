import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import test from "node:test";
import { isAbsolute, join, relative, resolve } from "node:path";

import * as publicApi from "../dist/index.js";
import { resolveGooseBinaryForRuntime } from "../dist/resolve-binary.js";

const supportedPlatforms = [
  ["darwin", "arm64", "@aaif/indra-binary-darwin-arm64", "indra"],
  ["darwin", "x64", "@aaif/indra-binary-darwin-x64", "indra"],
  ["linux", "arm64", "@aaif/indra-binary-linux-arm64", "indra"],
  ["linux", "x64", "@aaif/indra-binary-linux-x64", "indra"],
  ["win32", "x64", "@aaif/indra-binary-win32-x64", "indra.exe"],
];

function setGooseBinary(t, value) {
  const original = process.env.GOOSE_BINARY;
  process.env.GOOSE_BINARY = value;
  t.after(() => {
    if (original === undefined) {
      delete process.env.GOOSE_BINARY;
    } else {
      process.env.GOOSE_BINARY = original;
    }
  });
}

for (const [
  platform,
  arch,
  packageName,
  executableName,
] of supportedPlatforms) {
  test(`resolves ${platform}-${arch}`, () => {
    let resolvedSpecifier;
    let checkedPath;
    const fixturePackageRoot = join("/fixtures", packageName);

    const result = resolveGooseBinaryForRuntime(platform, arch, {
      resolvePackageJson(specifier) {
        resolvedSpecifier = specifier;
        return join(fixturePackageRoot, "package.json");
      },
      isFile(path) {
        checkedPath = path;
        return true;
      },
    });

    assert.equal(resolvedSpecifier, `${packageName}/package.json`);
    assert.equal(result, resolve(fixturePackageRoot, "bin", executableName));
    assert.equal(checkedPath, result);
    assert.equal(isAbsolute(result), true);
  });
}

test("exports only the public resolver from the package root", () => {
  assert.deepEqual(Object.keys(publicApi), ["resolveGooseBinary"]);
});

test("uses GOOSE_BINARY as an explicit override", (t) => {
  const directory = mkdtempSync(join(tmpdir(), "goose-acp-override-"));
  const binaryPath = join(directory, "goose");
  writeFileSync(binaryPath, "");
  setGooseBinary(t, relative(process.cwd(), binaryPath));

  t.after(() => {
    rmSync(directory, { recursive: true, force: true });
  });

  assert.equal(publicApi.resolveGooseBinary(), binaryPath);
});

test("rejects an invalid GOOSE_BINARY override", (t) => {
  setGooseBinary(t, "missing-goose-binary");

  assert.throws(
    () => publicApi.resolveGooseBinary(),
    /GOOSE_BINARY does not point to a file/,
  );
});

test("reports unsupported platform and architecture combinations", () => {
  assert.throws(
    () =>
      resolveGooseBinaryForRuntime("freebsd", "x64", {
        resolvePackageJson() {
          throw new Error("should not resolve a package");
        },
        isFile() {
          return false;
        },
      }),
    /No Goose npm binary is available for freebsd-x64/,
  );
});

test("reports a missing optional platform package", () => {
  assert.throws(
    () =>
      resolveGooseBinaryForRuntime("linux", "x64", {
        resolvePackageJson() {
          throw new Error("module not found");
        },
        isFile() {
          return false;
        },
      }),
    /Indra binary package @aaif\/indra-binary-linux-x64 is not installed/,
  );
});

test("reports a missing executable in an installed platform package", () => {
  assert.throws(
    () =>
      resolveGooseBinaryForRuntime("darwin", "arm64", {
        resolvePackageJson() {
          return join(
            "/fixtures",
            "@aaif/goose-binary-darwin-arm64/package.json",
          );
        },
        isFile() {
          return false;
        },
      }),
    /Indra executable from @aaif\/indra-binary-darwin-arm64 was not found/,
  );
});
