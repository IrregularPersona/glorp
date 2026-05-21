class FpsMonitor {
  constructor(sampleLimit = 600, sendMs = 500) {
    this.samples = [];
    this.last = performance.now();
    this.sampleLimit = sampleLimit;
    this.interval = null;

    this.loop = this.loop.bind(this);
    requestAnimationFrame(this.loop);
    this.interval = setInterval(() => this.postStats(), sendMs);
  }

  loop(now) {
    const dt = now - this.last;
    this.last = now;
    const fps = dt > 0 ? 1000 / dt : 0;
    this.samples.push(fps);
    if (this.samples.length > this.sampleLimit) this.samples.shift();
    requestAnimationFrame(this.loop);
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