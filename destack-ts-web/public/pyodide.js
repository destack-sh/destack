// Pyodide Web Worker

/**
 * @typedef {Object} InitRequest
 * @property {'init'} type
 */

/**
 * @typedef {Object} RunRequest
 * @property {'run'} type
 * @property {Object} data
 * @property {string} data.code
 */

/**
 * @typedef {InitRequest | RunRequest} PyodideRequest
 */

/**
 * @typedef {Object} StatusResponse
 * @property {'status'} type
 * @property {string} message
 */

/**
 * @typedef {Object} ResultResponse
 * @property {'result'} type
 * @property {string|null} result
 * @property {string} stdout
 * @property {number} duration
 */

/**
 * @typedef {Object} ErrorResponse
 * @property {'error'} type
 * @property {string} message
 * @property {number} duration
 */

/**
 * @typedef {StatusResponse | ReadyResponse | ResultResponse | ErrorResponse} PyodideResponse
 */

let pyodide = null;
let isInitialized = false;

/**
 * Sends a status message to the main thread
 * @param {string} message - Status message
 */
function sendStatus(message) {
  /** @type {StatusResponse} */
  const response = { type: 'status', message };
  self.postMessage(response);
}

/**
 * Sends a result message to the main thread
 * @param {string|null} result - The result value
 * @param {string} stdout - Captured stdout
 * @param {number} duration - Execution duration in milliseconds
 */
function sendResult(result, stdout, duration) {
  /** @type {ResultResponse} */
  const response = { type: 'result', result, stdout, duration };
  self.postMessage(response);
}

/**
 * Sends an error message to the main thread
 * @param {string} message - Error message
 */
function sendError(message, duration) {
  /** @type {ErrorResponse} */
  const response = { type: 'error', message, duration };
  self.postMessage(response);
}

/**
 * Initialize Pyodide
 */
async function initializePyodide() {
  try {
    sendStatus("INITIALIZING");
    
    // Import pyodide dynamically from assets (copied by Vite)
    const { loadPyodide } = await import('/assets/pyodide.mjs');
    
    sendStatus("INITIALIZING");
    pyodide = await loadPyodide({
      indexURL: "/assets/"
    });
    
    isInitialized = true;
    sendStatus("READY");
  } catch (error) {
    sendError(`Failed to initialize Pyodide: ${error.message}`);
  }
}

/**
 * Execute Python code
 * @param {string} code - Python code to execute
 */
async function runPython(code) {
  if (!isInitialized || !pyodide) {
    sendError('Pyodide is not initialized yet', 0);
    return;
  }

  sendStatus("RUNNING");
  const startTime = performance.now();

  try {
    const result = await pyodide.runPythonAsync(code);
    const endTime = performance.now();
    const duration = endTime - startTime;
    sendResult(result ? String(result) : null, '', duration);
  } catch (error) {
    const endTime = performance.now();
    const duration = endTime - startTime;
    sendError(`${error.message}`, duration);
  } finally {
    sendStatus("READY");
  }
}

/**
 * Handle messages from main thread
 * @param {MessageEvent} event - Message event
 */
self.onmessage = async function(event) {
  /** @type {PyodideRequest} */
  const request = event.data;
  
  switch (request.type) {
    case 'init':
      await initializePyodide();
      break;
    case 'run':
      await runPython(request.data.code);
      break;
    default:
      sendError(`Unknown message type: ${request.type}`);
  }
}; 