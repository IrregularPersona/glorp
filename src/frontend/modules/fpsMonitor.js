class FpsMonitor {
  constructor(sampleLimit = 600, sendMs = 100) {
    this.samples = [];
    this.sampleLimit = sampleLimit;
    this.gl = null;
    this.ext = null;
    this.pendingQuery = null;
    this._last = null;

    this.loop = this.loop.bind(this);
    requestAnimationFrame(this.loop); // starts immediately, no async needed
    this.interval = setInterval(() => this.postStats(), sendMs);
  }

  tryInitGL() {
    if (this.gl) return; // already initialized
    const canvas = document.querySelector('canvas');
    if (!canvas) return;
    const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
    if (!gl) return;
    this.gl = gl;
    this.ext = gl.getExtension('EXT_disjoint_timer_query_webgl2');
    if (this.ext) {
        log('[FpsMonitor] using GPU timer (EXT_disjoint_timer_query_webgl2)');
    } else {
        log('[FpsMonitor] GPU timer extension unavailable, falling back to rAF timing');
    }
  }

  loop(now) {
    this.tryInitGL(); // cheap check every frame, no-ops once gl is set

    if (this.ext && this.gl) {
      this.readPendingQuery();

      const query = this.gl.createQuery();
      this.gl.beginQuery(this.ext.TIME_ELAPSED_EXT, query);
      this.pendingQuery = { query };

      requestAnimationFrame(() => {
        this.gl.endQuery(this.ext.TIME_ELAPSED_EXT);
      });
    } else {
      const dt = this._last !== null ? now - this._last : 0;
      if (dt > 0) {
        this.samples.push(1000 / dt);
        if (this.samples.length > this.sampleLimit) this.samples.shift();
      }
    }

    this._last = now;
    requestAnimationFrame(this.loop);
  }

  readPendingQuery() {
    if (!this.pendingQuery) return;
    const { query } = this.pendingQuery;

    const available = this.gl.getQueryParameter(query, this.gl.QUERY_RESULT_AVAILABLE);
    const disjoint = this.gl.getParameter(this.ext.GPU_DISJOINT_EXT);

    if (available && !disjoint) {
      const gpuNs = this.gl.getQueryParameter(query, this.gl.QUERY_RESULT);
      const gpuMs = gpuNs / 1_000_000;
      const fps = 1000 / Math.max(gpuMs, 0.001);
      this.samples.push(fps);
      if (this.samples.length > this.sampleLimit) this.samples.shift();
    }

    if (available || disjoint) {
      this.gl.deleteQuery(query);
      this.pendingQuery = null;
    }
  }

  percentile(arr, p) {
    if (!arr.length) return 0;
    const s = [...arr].sort((a, b) => a - b);
    const idx = Math.max(0, Math.floor((p / 100) * s.length) - 1);
    return s[idx];
  }

  getStats() {
    if (!this.samples.length) return { current: 0, average: 0, low1: 0, low01: 0 };
    const avg = this.samples.reduce((a, b) => a + b, 0) / this.samples.length;
    return {
      current: Math.round(this.samples[this.samples.length - 1]),
      average: Math.round(avg),
      low1: Math.round(this.percentile(this.samples, 1)),
      low01: Math.round(this.percentile(this.samples, 0.1)),
    };
  }

  postStats() {
    const payload = { command: "fps-stats", data: this.getStats() };
    window.chrome.webview.postMessage(JSON.stringify(payload));
  }

  destroy() {
    if (this.interval) clearInterval(this.interval);
  }
}

new FpsMonitor();