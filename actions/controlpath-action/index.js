const core = require('@actions/core');
const exec = require('@actions/exec');
const tc = require('@actions/tool-cache');
const path = require('path');
const fs = require('fs');
const os = require('os');
const https = require('https');

async function run() {
  try {
    // Get inputs
    const definitionsFile = core.getInput('definitions-file') || 'flags.definitions.yaml';
    const deploymentFile = core.getInput('deployment-file');
    const environment = core.getInput('environment');
    const version = core.getInput('version') || 'latest';
    const skipValidation = core.getInput('skip-validation') === 'true';
    const skipCompilation = core.getInput('skip-compilation') === 'true';

    // Linux only - determine architecture
    const arch = os.arch();
    const platformName = arch === 'arm64' ? 'linux-aarch64' : 'linux-x86_64';
    const binaryName = 'controlpath';
    const archiveExt = '.tar.gz';

    core.info(`Platform: ${platformName}`);

    // Helper function to fetch latest release tag from GitHub API
    async function getLatestReleaseTag() {
      const latestReleaseUrl = 'https://api.github.com/repos/releaseworkshop/control-path/releases/latest';
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

    // Helper function to find binary in extracted directory
    function findBinary(dir) {
      // Try the expected binary name first
      const expectedPath = path.join(dir, binaryName);
      if (fs.existsSync(expectedPath)) {
        return expectedPath;
      }
      
      // Try platform-specific name (e.g., controlpath-linux-x86_64)
      const platformBinaryPath = path.join(dir, `controlpath-${platformName}`);
      if (fs.existsSync(platformBinaryPath)) {
        return platformBinaryPath;
      }
      
      // Search for any file matching controlpath* pattern
      const files = fs.readdirSync(dir);
      const controlpathFile = files.find(f => f.startsWith('controlpath'));
      if (controlpathFile) {
        return path.join(dir, controlpathFile);
      }
      
      throw new Error(`Binary not found in extracted directory: ${dir}`);
    }

    // Consolidated function to download and install CLI
    async function downloadAndInstallCli(versionTag) {
      // Check cache first
      let cachedPath = tc.find('controlpath', versionTag);
      if (cachedPath) {
        core.info(`Using cached CLI from: ${cachedPath}`);
        const cachedBinary = path.join(cachedPath, binaryName);
        if (fs.existsSync(cachedBinary)) {
          return cachedBinary;
        }
        // Try to find binary in cached directory
        const foundBinary = findBinary(cachedPath);
        if (fs.existsSync(foundBinary)) {
          return foundBinary;
        }
      }

      // Download the CLI
      const downloadUrl = `https://github.com/releaseworkshop/control-path/releases/download/${versionTag}/controlpath-${versionTag}-${platformName}${archiveExt}`;
      core.info(`Downloading Control Path CLI ${versionTag} from: ${downloadUrl}`);
      
      const downloadPath = await tc.downloadTool(downloadUrl);
      core.info(`Downloaded to: ${downloadPath}`);
      
      // Extract archive
      const extractedPath = await tc.extractTar(downloadPath);
      
      // Find binary
      const cliPath = findBinary(extractedPath);
      core.info(`Found binary at: ${cliPath}`);
      
      // Make executable
      fs.chmodSync(cliPath, '755');
      
      // Cache the tool
      cachedPath = await tc.cacheFile(cliPath, binaryName, 'controlpath', versionTag);
      return path.join(cachedPath, binaryName);
    }

    // Determine version tag
    let versionTag = version;
    if (version === 'latest') {
      try {
        versionTag = await getLatestReleaseTag();
        core.info(`Latest release: ${versionTag}`);
      } catch (error) {
        core.warning(`Failed to fetch latest release from GitHub API: ${error.message}`);
        throw new Error(`Unable to determine latest version. Please specify a version tag explicitly.`);
      }
    }

    // Download and install CLI
    const cliPath = await downloadAndInstallCli(versionTag);
    core.info(`Control Path CLI installed at: ${cliPath}`);
    core.addPath(path.dirname(cliPath));

    // Validate flags
    if (!skipValidation) {
      core.info('Validating flag definitions...');
      let validateArgs = [];
      
      if (definitionsFile && definitionsFile !== 'flags.definitions.yaml') {
        validateArgs.push('--definitions', definitionsFile);
      }
      
      if (deploymentFile) {
        validateArgs.push('--deployment', deploymentFile);
      } else if (environment) {
        validateArgs.push('--env', environment);
      } else {
        // Auto-detect if no specific file provided
        validateArgs.push('--all');
      }
      
      let validateExitCode = await exec.exec(cliPath, ['validate', ...validateArgs]);
      if (validateExitCode !== 0) {
        core.setFailed('Validation failed');
        return;
      }
      core.info('✓ Validation passed');
    } else {
      core.info('Skipping validation (skip-validation=true)');
    }

    // Compile flags
    if (!skipCompilation) {
      core.info('Compiling flag definitions...');
      let compileArgs = [];
      
      if (definitionsFile && definitionsFile !== 'flags.definitions.yaml') {
        compileArgs.push('--definitions', definitionsFile);
      }
      
      if (deploymentFile) {
        compileArgs.push('--deployment', deploymentFile);
      } else if (environment) {
        compileArgs.push('--env', environment);
      } else {
        core.setFailed('Either deployment-file or environment must be provided for compilation');
        return;
      }
      
      let compileExitCode = await exec.exec(cliPath, ['compile', ...compileArgs]);
      if (compileExitCode !== 0) {
        core.setFailed('Compilation failed');
        return;
      }
      
      // Determine output path
      let artifactPath;
      if (environment) {
        artifactPath = `.controlpath/${environment}.ast`;
      } else if (deploymentFile) {
        // Infer from deployment file path
        const deploymentDir = path.dirname(deploymentFile);
        const deploymentBase = path.basename(deploymentFile, '.deployment.yaml');
        artifactPath = path.join(deploymentDir, `${deploymentBase}.ast`);
      } else {
        artifactPath = 'deployment.ast';
      }
      
      // Check if artifact exists
      if (fs.existsSync(artifactPath)) {
        core.info(`✓ Compiled artifact: ${artifactPath}`);
        core.setOutput('compiled-artifact-path', artifactPath);
      } else {
        core.warning(`Compiled artifact not found at expected path: ${artifactPath}`);
      }
    } else {
      core.info('Skipping compilation (skip-compilation=true)');
    }

  } catch (error) {
    core.setFailed(error.message);
  }
}

run();
