class FpsMonitor {
	constructor() {
		this.ingameFPS = null;
		this.menuFPS = null;
		this.interval = null;
		this.listener = null;

		window.glorp.settings.toggleFpsMonitor = (enabled) => this.toggle(enabled);
		this.toggle(true);
	}

	applyFpsDisplay(element) {
		if (!element) return;
		Object.defineProperty(element, "textContent", {
			set: () => {},
			configurable: true,
		});
	}

	async toggle(enabled) {
		[this.ingameFPS, this.menuFPS] = await Promise.all([
			waitForElement("#ingameFPS"),
			waitForElement("#menuFPS"),
		]);
		if (enabled) {
			this.applyFpsDisplay(this.ingameFPS);
			this.applyFpsDisplay(this.menuFPS);
			this.interval = setInterval(() => {
				window.chrome.webview.postMessage("fps");
			}, 100);

			this.listener = (event) => {
				if (event.data.fpsInfo === undefined) return;
				this.ingameFPS.innerText = event.data.fpsInfo;
				this.menuFPS.innerText = event.data.fpsInfo;
			};
			window.chrome.webview.addEventListener("message", this.listener);
		} else {
			clearInterval(this.interval);
			if (this.listener) {
				window.chrome.webview.removeEventListener("message", this.listener);
			}
			delete this.ingameFPS.textContent;
			delete this.menuFPS.textContent;
		}
	}
}

new FpsMonitor();