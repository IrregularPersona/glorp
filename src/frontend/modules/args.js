const automateCompHost = async (params) => {
	window.openHostWindow(false, 1);
	await waitForElement(".hostTb0");
	let mapCheckbox = null;
	mapCheckbox = document.querySelector(`#${params.mapId}`);

	if (!mapCheckbox) {
		const allMapNameElements = document.querySelectorAll(".hostMap .hostMapName");
		const targetNameElement = Array.from(allMapNameElements).find(
			(el) => el.innerText.trim().toLowerCase() === params.mapId.toLowerCase(),
		);
		if (targetNameElement) mapCheckbox = targetNameElement.parentElement.querySelector('input[type="checkbox"]');
	}

	if (!mapCheckbox) return;

	if (!mapCheckbox.checked) mapCheckbox.click();

	windows[7].switchTab(2);

	const team1Input = await waitForElement("#customSnameTeam1");
	team1Input.value = params.team1Name;
	const team2Input = await waitForElement("#customSnameTeam2");
	team2Input.value = params.team2Name;

	const teamSizeSelect = await waitForElement("#customStmSize");

	const teamSizeMap = {
		"1v1": "0",
		"2v2": "1",
		"3v3": "2",
		"4v4": "3",
	};

	if (params.team1Players) {
		const compRosterT1 = await waitForElement("#compRosterT1");
		compRosterT1.value = params.team1Players;
	}
	if (params.team2Players) {
		const compRosterT2 = await waitForElement("#compRosterT2");
		compRosterT2.value = params.team2Players;
	}

	if (params.spectators) {
		const compSpectators = await waitForElement("#compRosterSpecs");
		compSpectators.value = params.spectators;
	}

	const finalTeamSize = teamSizeMap[params.teamSize] || params.teamSize;
	teamSizeSelect.value = finalTeamSize;

	if (params.webhook) {
		try {
			const webhookInput = await waitForElement("#customSwebhook");
			webhookInput.value = decodeURIComponent(params.webhook);
		} catch {
			/* */
		}
	}
	window.createPrivateRoom();
};


const changeRegion = async (region) => {
	const regionMap = {
		FRA: "de-fra",
		SV: "us-ca-sv",
		SYD: "au-syd",
		TOK: "jb-hnd",
		SIN: "sgp",
		NY: "us-nj",
		MUM: "as-mb",
		DAL: "us-tx",
		BR: "brz",
		ME: "me-bhn",
	};

	// basic sanitation
	const normalizedRegion = regionMap[region.toUpperCase()] || region;

	window.showWindow(1);

	const selectRoot = document.querySelector("select.inputGrey2");
	if (!selectRoot) {
		if (typeof window.setSetting === "function") {
			window.setSetting("defaultRegion", normalizedRegion);
		}
		return;
	}

	const regionValues = Object.values(regionMap);
	const regionSelect = Array.from(document.querySelectorAll("select.inputGrey2")).find((select) =>
    Array.from(select.options).some((opt) => regionValues.includes(opt.value))) || selectRoot;

	if (regionSelect && regionSelect.value !== normalizedRegion) {
		const optionIndex = Array.from(regionSelect.options).findIndex((opt) => opt.value === normalizedRegion);
		if (optionIndex !== -1) {
			regionSelect.selectedIndex = optionIndex;
			regionSelect.dispatchEvent(new Event("change", { bubbles: true, cancelable: true }));
		}
	}

	// cant manually invoke the change handler 
	// if only you could :(
	
	// if (typeof window.setSetting === "function") {
	// 	window.setSetting("defaultRegion", normalizedRegion);
	// }

	window.showWindow(1);
};

window.glorp.parseArgs = async (args) => {
    // log("full args:", args);
    args = args.split(" ");
    for (const arg of args) {
        if (arg.includes("action=host-comp")) {
            // log("Matched arg:", arg);
            const url = new URL(arg);
            const params = Object.fromEntries(url.searchParams.entries());
            // log("params:", params);
            // log("region check:", params.region);
            if (params.region) await changeRegion(params.region);
            await automateCompHost(params);
        }
    }
};

window.chrome.webview.addEventListener("message", async (event) => {
	if (!event.data.args) return;
	await window.glorp.parseArgs(event.data.args);
});
