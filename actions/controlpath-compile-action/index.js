const core = require('@actions/core');
const exec = require('@actions/exec');
const tc = require('@actions/tool-cache');
const path = require('path');
const fs = require('fs');
const os = require('os');
const https = require('https');

const REPOSITORY = 'releaseworkshop/control-path';

async function run() {
  let originalCwd;

  try {
    const environment = core.getInput('environment');
    const version = core.getInput('version') || 'latest';
    const skipValidation = core.getInput('skip-validation') === 'true';
    const skipCompilation = core.getInput('skip-compilation') === 'true';
    const workingDirectory = core.getInput('working-directory');

    if (workingDirectory) {
      originalCwd = process.cwd();
      const fullWorkingDir = path.resolve(workingDirectory);
      if (!fs.existsSync(fullWorkingDir)) {
        throw new Error(`Working directory does not exist: ${fullWorkingDir}`);
      }
      if (!fs.statSync(fullWorkingDir).isDirectory()) {
        throw new Error(`Working directory is not a directory: ${fullWorkingDir}`);
      }
      process.chdir(fullWorkingDir);
      core.info(`Changed working directory to: ${fullWorkingDir}`);
    }

    if (skipValidation && skipCompilation) {
      core.setFailed('Both skip-validation and skip-compilation cannot be true. At least one step must run.');
      return;
    }

    if (!fs.existsSync('control-path.yaml')) {
      core.setFailed('control-path.yaml not found in the working directory');
      return;
    }

    const arch = os.arch();
    const platformName = arch === 'arm64' ? 'linux-aarch64' : 'linux-x86_64';
    const binaryName = 'controlpath';
    const archiveExt = '.tar.gz';

    core.info(`Platform: ${platformName}`);

    async function getLatestReleaseTag() {
      const latestReleaseUrl = `https://api.github.com/repos/${REPOSITORY}/releases/latest`;
      core.info(`Fetching latest release info from: ${latestReleaseUrl}`);

      return new Promise((resolve, reject) => {
        let req;
        const timeout = setTimeout(() => {
          if (req) req.destroy();
          reject(new Error('Request timeout'));
        }, 10000);

        req = https.get(latestReleaseUrl, {
          headers: {
            'User-Agent': 'controlpath-action'
          }
        }, (res) => {
          if (res.statusCode !== 200) {
            clearTimeout(timeout);
            reject(new Error(`GitHub API returned status ${res.statusCode}`));
            return;
          }

          let data = '';
          res.on('data', (chunk) => { data += chunk; });
          res.on('end', () => {
            clearTimeout(timeout);
            try {
              const release = JSON.parse(data);
              resolve(release.tag_name);
            } catch (e) {
              reject(new Error(`Failed to parse latest release info: ${e.message}`));
            }
          });
        }).on('error', (err) => {
          clearTimeout(timeout);
          reject(err);
        });
      });
    }

    function findBinary(dir) {
      const expectedPath = path.join(dir, binaryName);
      if (fs.existsSync(expectedPath)) {
        return expectedPath;
      }

      const platformBinaryPath = path.join(dir, `controlpath-${platformName}`);
      if (fs.existsSync(platformBinaryPath)) {
        return platformBinaryPath;
      }

      const files = fs.readdirSync(dir);
      const controlpathFile = files.find(f => f.startsWith('controlpath'));
      if (controlpathFile) {
        return path.join(dir, controlpathFile);
      }

      throw new Error(`Binary not found in extracted directory: ${dir}`);
    }

    async function downloadAndInstallCli(versionTag) {
      let cachedPath = tc.find('controlpath', versionTag);
      if (cachedPath) {
        core.info(`Using cached CLI from: ${cachedPath}`);
        const cachedBinary = path.join(cachedPath, binaryName);
        if (fs.existsSync(cachedBinary)) {
          return cachedBinary;
        }
        const foundBinary = findBinary(cachedPath);
        if (fs.existsSync(foundBinary)) {
          return foundBinary;
        }
      }

      const downloadUrl = `https://github.com/${REPOSITORY}/releases/download/${versionTag}/controlpath-${versionTag}-${platformName}${archiveExt}`;
      core.info(`Downloading Control Path CLI ${versionTag} from: ${downloadUrl}`);

      const downloadPath = await tc.downloadTool(downloadUrl);
      core.info(`Downloaded to: ${downloadPath}`);

      const extractedPath = await tc.extractTar(downloadPath);
      const cliPath = findBinary(extractedPath);
      core.info(`Found binary at: ${cliPath}`);

      fs.chmodSync(cliPath, 0o755);

      cachedPath = await tc.cacheFile(cliPath, binaryName, 'controlpath', versionTag);
      return path.join(cachedPath, binaryName);
    }

    let versionTag = version;
    if (version === 'latest') {
      try {
        versionTag = await getLatestReleaseTag();
        core.info(`Latest release: ${versionTag}`);
      } catch (error) {
        core.warning(`Failed to fetch latest release from GitHub API: ${error.message}`);
        throw new Error('Unable to determine latest version. Please specify a version tag explicitly.');
      }
    }

    const cliPath = await downloadAndInstallCli(versionTag);
    core.info(`Control Path CLI installed at: ${cliPath}`);
    core.addPath(path.dirname(cliPath));

    if (!skipValidation) {
      core.info('Validating catalog...');
      const validateArgs = environment ? ['--env', environment] : ['--all'];
      const validateExitCode = await exec.exec(cliPath, ['validate', ...validateArgs]);
      if (validateExitCode !== 0) {
        core.setFailed('Validation failed');
        return;
      }
      core.info('✓ Validation passed');
    } else {
      core.info('Skipping validation (skip-validation=true)');
    }

    if (!skipCompilation) {
      if (!environment) {
        core.setFailed('environment input is required for compilation');
        return;
      }

      core.info(`Compiling catalog for environment: ${environment}`);
      const compileExitCode = await exec.exec(cliPath, ['compile', '--env', environment]);
      if (compileExitCode !== 0) {
        core.setFailed('Compilation failed');
        return;
      }

      const artifactPath = path.join('.controlpath', `${environment}.ast`);
      if (fs.existsSync(artifactPath)) {
        core.info(`✓ Compiled artifact: ${artifactPath}`);
        core.setOutput('compiled-artifact-path', artifactPath);
      } else {
        core.setFailed(`Compiled artifact not found at expected path: ${artifactPath}. Compilation may have failed or written to a different location.`);
        return;
      }
    } else {
      core.info('Skipping compilation (skip-compilation=true)');
    }

  } catch (error) {
    core.setFailed(`Control Path Compile Action failed: ${error.message}`);
    if (core.isDebug()) {
      core.debug(error.stack);
    }
  } finally {
    if (originalCwd) {
      process.chdir(originalCwd);
    }
  }
}

run();
