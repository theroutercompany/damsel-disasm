const state = {
  imageId: null,
  fileName: null,
  architecture: null,
  entryPoint: null,
  sections: [],
  defaultSection: null,
  disasm: null,
  selectedInstructionIndex: 0,
  symbolLookupTimer: null,
};

const elements = {
  body: document.body,
  dropzone: document.getElementById("dropzone"),
  fileInput: document.getElementById("file-input"),
  fileSummary: document.getElementById("file-summary"),
  banner: document.getElementById("banner"),
  sectionSelect: document.getElementById("section-select"),
  symbolInput: document.getElementById("symbol-input"),
  symbolSuggestions: document.getElementById("symbol-suggestions"),
  addressInput: document.getElementById("address-input"),
  fromInput: document.getElementById("from-input"),
  toInput: document.getElementById("to-input"),
  bytesInput: document.getElementById("bytes-input"),
  limitInput: document.getElementById("limit-input"),
  annotationsToggle: document.getElementById("annotations-toggle"),
  valueFlowToggle: document.getElementById("value-flow-toggle"),
  analysisToggle: document.getElementById("analysis-toggle"),
  runButton: document.getElementById("run-button"),
  resultTitle: document.getElementById("result-title"),
  resultMeta: document.getElementById("result-meta"),
  instructionRows: document.getElementById("instruction-rows"),
  inspectorTitle: document.getElementById("inspector-title"),
  inspectorContent: document.getElementById("inspector-content"),
  sectionGroup: document.getElementById("section-group"),
  symbolGroup: document.getElementById("symbol-group"),
  addressGroup: document.getElementById("address-group"),
  listingPane: document.querySelector(".canvas-pane"),
  inspectorPane: document.querySelector(".inspector-pane"),
  controlsPane: document.querySelector(".controls-pane"),
};

function currentTargetKind() {
  return document.querySelector('input[name="target-kind"]:checked').value;
}

function setBanner(message, tone = "error") {
  if (!message) {
    elements.banner.hidden = true;
    elements.banner.textContent = "";
    elements.banner.dataset.tone = "";
    return;
  }

  elements.banner.hidden = false;
  elements.banner.textContent = message;
  elements.banner.dataset.tone = tone;
}

function syncWorkbenchState() {
  const selectedInstruction = state.disasm?.instructions?.[state.selectedInstructionIndex];
  elements.body.dataset.hasImage = state.imageId ? "true" : "false";
  elements.body.dataset.hasSelection = selectedInstruction ? "true" : "false";
}

function setWorkbenchPhase(phase) {
  elements.body.dataset.phase = phase;
}

function pulseSurface(node) {
  if (!node) {
    return;
  }

  node.classList.remove("is-refreshing");
  void node.offsetWidth;
  node.classList.add("is-refreshing");
  window.setTimeout(() => {
    node.classList.remove("is-refreshing");
  }, 220);
}

function updateFileSummary() {
  const label = elements.fileSummary.querySelector(".summary-label");
  const value = elements.fileSummary.querySelector(".summary-value");

  if (!state.imageId) {
    label.textContent = "Idle";
    value.textContent = "Awaiting local binary";
    return;
  }

  label.textContent = state.fileName;
  value.textContent = `${state.architecture} · entry ${formatAddress(state.entryPoint)} · ${state.sections.length} sections`;
}

function updateTargetControls() {
  const targetKind = currentTargetKind();
  elements.sectionGroup.hidden = targetKind !== "section";
  elements.symbolGroup.hidden = targetKind !== "symbol";
  elements.addressGroup.hidden = targetKind !== "address";
  elements.fromInput.disabled = targetKind === "address";
}

function formatAddress(value) {
  const bigint = addressBigInt(value);
  if (bigint === null) {
    return "n/a";
  }

  return `0x${bigint.toString(16)}`;
}

function addressBigInt(value) {
  if (value === null || value === undefined) {
    return null;
  }

  if (typeof value === "bigint") {
    return value >= 0n ? value : null;
  }

  if (typeof value === "number") {
    if (!Number.isInteger(value) || value < 0) {
      return null;
    }
    return BigInt(value);
  }

  if (typeof value !== "string") {
    return null;
  }

  const normalized = value.trim().replaceAll("_", "");
  if (!/^0x[0-9a-f]+$/i.test(normalized) && !/^[0-9]+$/.test(normalized)) {
    return null;
  }

  const bigint = BigInt(normalized);
  return bigint <= 0xffff_ffff_ffff_ffffn ? bigint : null;
}

function parseOptionalInteger(input, label) {
  const rawValue = input.value.trim();
  if (!rawValue) {
    return { value: null, bigint: null, error: null };
  }

  const normalized = rawValue.replaceAll("_", "");
  const isHex = /^0x[0-9a-f]+$/i.test(normalized);
  const isDecimal = /^[0-9]+$/.test(normalized);
  if (!isHex && !isDecimal) {
    return { value: null, bigint: null, error: `${label} must be a decimal or hex integer.` };
  }

  const bigint = BigInt(normalized);
  if (bigint > 0xffff_ffff_ffff_ffffn) {
    return { value: null, bigint: null, error: `${label} must fit in an unsigned 64-bit integer.` };
  }

  return { value: normalized, bigint, error: null };
}

function requestIntegerValue(input) {
  return parseOptionalInteger(input, "Value").value;
}

function parsedIntegerValue(input) {
  return parseOptionalInteger(input, "Value").bigint;
}

function currentStartAddressHint(targetKind) {
  if (targetKind === "address" && elements.addressInput.value.trim()) {
    return parsedIntegerValue(elements.addressInput);
  }

  if (targetKind === "section") {
    const section = state.sections.find((entry) => entry.name === elements.sectionSelect.value);
    return section ? addressBigInt(section.address) : null;
  }

  if (elements.fromInput.value.trim()) {
    return parsedIntegerValue(elements.fromInput);
  }

  return null;
}

function validateInputs() {
  const targetKind = currentTargetKind();
  if (!state.imageId) {
    return "Upload a Mach-O before requesting disassembly.";
  }

  if (targetKind === "section" && !elements.sectionSelect.value) {
    return "Choose a section target.";
  }

  if (targetKind === "symbol" && !elements.symbolInput.value.trim()) {
    return "Enter a symbol name.";
  }

  if (targetKind === "address" && !elements.addressInput.value.trim()) {
    return "Enter a decode address.";
  }

  const address =
    targetKind === "address"
      ? parseOptionalInteger(elements.addressInput, "Address")
      : { value: null, error: null };
  const bytes = parseOptionalInteger(elements.bytesInput, "Bytes");
  const limit = parseOptionalInteger(elements.limitInput, "Limit");
  const from = parseOptionalInteger(elements.fromInput, "From");
  const to = parseOptionalInteger(elements.toInput, "To");
  const startHint = currentStartAddressHint(targetKind);
  for (const parsed of [address, bytes, limit, from, to]) {
    if (parsed.error) {
      return parsed.error;
    }
  }

  if (targetKind === "address" && address.value === null) {
    return "Enter a valid decimal or hex decode address.";
  }

  if (bytes.bigint !== null && bytes.bigint <= 0n) {
    return "Bytes must be greater than 0.";
  }

  if (limit.bigint !== null && limit.bigint <= 0n) {
    return "Limit must be greater than 0.";
  }

  if (bytes.value !== null && limit.value !== null) {
    return "Bytes cannot be combined with limit.";
  }

  if (to.value !== null && (bytes.value !== null || limit.value !== null)) {
    return "To cannot be combined with bytes or limit.";
  }

  if (from.bigint !== null && to.bigint !== null && to.bigint <= from.bigint) {
    return "To must be greater than from.";
  }

  if (startHint !== null && to.bigint !== null && to.bigint <= startHint) {
    return "To must be greater than the decode start.";
  }

  return null;
}

function buildDisasmRequest() {
  const targetKind = currentTargetKind();
  const window = {
    from: targetKind === "address" ? undefined : requestIntegerValue(elements.fromInput),
    to: requestIntegerValue(elements.toInput),
    bytes: requestIntegerValue(elements.bytesInput),
    limit: requestIntegerValue(elements.limitInput),
  };
  const address = parseOptionalInteger(elements.addressInput, "Address");
  const value =
    targetKind === "section"
      ? elements.sectionSelect.value
      : targetKind === "symbol"
        ? elements.symbolInput.value.trim()
        : address.value;

  return {
    target: {
      kind: targetKind,
      value,
    },
    window,
    options: {
      includeAnnotations: elements.annotationsToggle.checked,
      includeValueFlow: elements.valueFlowToggle.checked,
      includeAnalysis: elements.analysisToggle.checked,
    },
  };
}

async function loadImage(file) {
  setBanner("");
  setWorkbenchPhase("uploading");
  pulseSurface(elements.controlsPane);

  const formData = new FormData();
  formData.append("file", file, file.name || "uploaded-macho");

  try {
    const response = await fetch("/api/images", {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      return handleApiError(response);
    }

    const payload = await response.json();
    state.imageId = payload.imageId;
    state.fileName = payload.fileName;
    state.architecture = payload.architecture;
    state.entryPoint = payload.entryPoint;
    state.sections = payload.sections || [];
    state.defaultSection = payload.defaultSection;
    populateSections();
    updateFileSummary();
    syncWorkbenchState();

    if (payload.defaultSection) {
      elements.sectionSelect.value = payload.defaultSection;
    }

    pulseSurface(elements.controlsPane);
    await runDisasm();
  } finally {
    setWorkbenchPhase("idle");
  }
}

function populateSections() {
  elements.sectionSelect.innerHTML = "";
  for (const section of state.sections) {
    const option = document.createElement("option");
    option.value = section.name;
    option.textContent = `${section.name}${section.executable ? " · exec" : ""}`;
    elements.sectionSelect.append(option);
  }
}

async function fetchSymbolSuggestions() {
  if (!state.imageId || !elements.symbolInput.value.trim()) {
    elements.symbolSuggestions.innerHTML = "";
    return;
  }

  const query = new URLSearchParams({
    q: elements.symbolInput.value.trim(),
    limit: "20",
  });
  const response = await fetch(`/api/images/${state.imageId}/symbols?${query.toString()}`);
  if (!response.ok) {
    return;
  }

  const payload = await response.json();
  elements.symbolSuggestions.innerHTML = "";
  for (const symbol of payload.symbols || []) {
    const option = document.createElement("option");
    option.value = symbol;
    elements.symbolSuggestions.append(option);
  }
}

async function runDisasm() {
  const validation = validateInputs();
  if (validation) {
    setBanner(validation);
    return;
  }

  setBanner("");
  setWorkbenchPhase("decoding");

  try {
    const response = await fetch(`/api/images/${state.imageId}/disasm`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(buildDisasmRequest()),
    });

    if (!response.ok) {
      return handleApiError(response);
    }

    state.disasm = await response.json();
    state.selectedInstructionIndex = 0;
    renderDisasm();
    renderInspector();
    syncWorkbenchState();
    pulseSurface(elements.listingPane);
    pulseSurface(elements.inspectorPane);
  } finally {
    setWorkbenchPhase("idle");
  }
}

async function handleApiError(response) {
  const payload = await response.json().catch(() => null);
  const message = payload
    ? `${payload.code}: ${payload.message}`
    : `Request failed with status ${response.status}`;
  setBanner(message);
}

function renderDisasm() {
  const data = state.disasm;
  if (!data || !Array.isArray(data.instructions) || data.instructions.length === 0) {
    elements.resultTitle.textContent = "No target";
    elements.resultMeta.textContent = "Load a local binary to decode a target.";
    elements.instructionRows.innerHTML = `
      <tr class="empty-row">
        <td colspan="3">This target did not produce any decoded instructions.</td>
      </tr>
    `;
    return;
  }

  elements.resultTitle.textContent = data.target;
  const analysisMeta = renderResultAnalysisMeta(data.analysis);
  elements.resultMeta.textContent = `${data.instruction_count} rows · ${data.stop_reason}${analysisMeta}`;
  elements.instructionRows.innerHTML = "";

  data.instructions.forEach((instruction, index) => {
    const row = document.createElement("tr");
    row.classList.toggle("is-selected", index === state.selectedInstructionIndex);
    row.addEventListener("click", () => {
      state.selectedInstructionIndex = index;
      renderDisasm();
      renderInspector();
      syncWorkbenchState();
    });

    const targetRef = (instruction.references || []).find(
      (reference) =>
        (reference.type === "call" || reference.type === "branch") &&
        reference.target !== undefined,
    );
    const jumpChip = targetRef ? renderInstructionJumpChip(targetRef.target) : "";

    row.innerHTML = `
      <td class="instruction-address mono">${formatAddress(instruction.address)}</td>
      <td class="instruction-opcode mono">${formatAddress(instruction.opcode)}</td>
      <td>
        <div class="instruction-main">
          <div class="instruction-rendered mono">${escapeHtml(instruction.rendered)}</div>
          ${jumpChip}
        </div>
      </td>
    `;

    const jumpButton = row.querySelector("[data-target]");
    if (jumpButton) {
      jumpButton.addEventListener("click", async (event) => {
        event.stopPropagation();
        await jumpToAddress(jumpButton.dataset.target);
      });
    }

    elements.instructionRows.append(row);
  });
}

function renderResultAnalysisMeta(analysis) {
  if (!analysis) {
    return "";
  }

  const summary = analysis.summary || {};
  const unresolved = summary.unresolved_indirect_count ?? 0;
  const unresolvedText = unresolved ? ` · ${unresolved} unresolved indirect` : "";
  return ` · ${summary.basic_block_count ?? 0} blocks · ${summary.edge_count ?? 0} edges · ${summary.direct_call_count ?? 0} calls · ${summary.data_reference_count ?? 0} data refs · ${summary.import_count ?? 0} imports${unresolvedText}`;
}

function renderInspector() {
  const data = state.disasm;
  const instruction = data?.instructions?.[state.selectedInstructionIndex];
  if (!instruction) {
    elements.inspectorTitle.textContent = "No selection";
    elements.inspectorContent.innerHTML =
      '<p class="inspector-empty">Select an instruction to inspect references and recovered values.</p>';
    return;
  }

  elements.inspectorTitle.textContent = `${instruction.mnemonic} · ${formatAddress(
    instruction.address,
  )}`;

  const blocks = [];
  blocks.push(
    renderDetailBlock("Instruction", [
      ["Address", formatAddress(instruction.address)],
      ["Opcode", formatAddress(instruction.opcode)],
      ["Rendered", instruction.rendered],
    ]),
  );
  blocks.push(renderObjectBlock("Operands", structuredOperandEntries(instruction)));
  blocks.push(renderReferenceBlock(instruction.references || []));
  blocks.push(renderObjectBlock("Annotations", instruction.annotations || []));
  blocks.push(renderObjectBlock("Recovered Values", instruction.recovered_values || []));
  if (data.analysis) {
    blocks.push(renderAnalysisBlock(data.analysis));
  }
  elements.inspectorContent.innerHTML = blocks.join("");

  elements.inspectorContent
    .querySelectorAll("[data-jump-target]")
    .forEach((button) => {
      button.addEventListener("click", async () => {
        await jumpToAddress(button.dataset.jumpTarget);
      });
    });
}

function structuredOperandEntries(instruction) {
  if (Array.isArray(instruction.structured_operands)) {
    return instruction.structured_operands;
  }

  return (instruction.operands || []).map((display, index) => ({
    index,
    display,
  }));
}

function renderInstructionJumpChip(value) {
  const label = formatAddress(value);
  if (label === "n/a") {
    return "";
  }

  return `<button class="instruction-target" type="button" data-target="${escapeHtml(label)}">jump ${escapeHtml(label)}</button>`;
}

function renderAnalysisBlock(analysis) {
  const metrics = analysis.metrics || {};
  const blocks = analysis.basic_blocks || [];
  const blockById = buildBlockMap(blocks);
  return `
    <section class="inspector-block analysis-block">
      <h3>Summary Graph</h3>
      <div class="detail-list">
        ${renderDetailRows([
          ["Blocks", analysis.summary.basic_block_count],
          ["Edges", analysis.summary.edge_count],
          ["Calls", analysis.summary.direct_call_count],
          ["Indirect Calls", analysis.summary.indirect_call_count],
          ["Branches", analysis.summary.branch_count],
          ["Returns", analysis.summary.return_count],
          ["Data Refs", analysis.summary.data_reference_count],
          ["Imports", analysis.summary.import_count],
          ["ObjC Selectors", analysis.summary.objc_selector_count ?? 0],
          ["ObjC Classes", analysis.summary.objc_class_count ?? 0],
          ["ObjC Method Lists", analysis.summary.objc_method_list_count ?? 0],
          ["ObjC Sends", analysis.summary.objc_message_send_count ?? 0],
          ["Cache Links", analysis.summary.cache_link_count],
          ["Recovered Values", analysis.summary.recovered_value_count],
          ["Jump Tables", analysis.summary.jump_table_count],
          ["Unresolved Indirect", analysis.summary.unresolved_indirect_count],
          ["Decoded Bytes", metrics.decoded_byte_count ?? 0],
          ["References", metrics.reference_count ?? 0],
          ["Annotations", metrics.annotation_count ?? 0],
          ["Max Block Insn", metrics.max_basic_block_instruction_count ?? 0],
          ["Intra-window Edges", metrics.intra_window_edge_count ?? 0],
          ["External Target Edges", metrics.external_target_edge_count ?? 0],
          ["Unresolved Target Edges", metrics.unresolved_target_edge_count ?? 0],
        ])}
      </div>
      ${renderAnalysisOverview(analysis.overview || {})}
      ${renderAnalysisBlocks(blocks, blockById)}
      ${renderAnalysisEdges(analysis.edges || [], blockById)}
      ${renderAnalysisTargetUses("Direct Calls", analysis.direct_calls || [])}
      ${renderAnalysisTargetUses("Branch Targets", analysis.branch_targets || [])}
      ${renderAnalysisTargetUses("Data References", analysis.data_references || [])}
      ${renderAnalysisIndirectControls(analysis.indirect_controls || [])}
      ${renderAnalysisJumpTables(analysis.jump_tables || [])}
      ${renderAnalysisRecoveredValues(analysis.recovered_values || [])}
      ${renderAnalysisImports(analysis.imports || [], analysis.cache_links || [])}
      ${renderAnalysisObjcReferences(analysis.objc_references || [])}
    </section>
  `;
}

function renderAnalysisOverview(overview) {
  const sections = [
    renderOverviewTargets("Calls", overview.call_targets || []),
    renderOverviewTargets("Branches", overview.branch_targets || []),
    renderOverviewIndirectControls(overview.indirect_controls || []),
    renderOverviewJumpTables(overview.jump_tables || []),
    renderOverviewTargets("Data", overview.data_targets || []),
    renderOverviewImports(overview.imports || []),
    renderOverviewCacheLinks(overview.cache_links || []),
    renderOverviewObjc(overview.objc_references || []),
    renderOverviewValues(overview.recovered_values || []),
  ].filter(Boolean);

  if (!sections.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Overview</div>
      ${sections.join("")}
    </div>
  `;
}

function renderOverviewTargets(label, rows) {
  if (!rows.length) {
    return "";
  }

  return rows
    .map(
      (entry) => `
        <div class="analysis-row">
          <span class="mono">${escapeHtml(label)}</span>
          ${renderJumpButton(entry.first_instruction_address)}
          ${renderJumpButton(entry.target_address)}
          <span class="mono">${escapeHtml(String(entry.count))}x</span>
        </div>
      `,
    )
    .join("");
}

function renderOverviewImports(rows) {
  if (!rows.length) {
    return "";
  }

  return rows
    .map(
      (entry) => `
        <div class="analysis-row">
          <span class="mono">Import</span>
          ${renderJumpButton(entry.first_instruction_address)}
          <span class="mono">${escapeHtml(entry.dylib)}</span>
          <span class="mono">${escapeHtml(entry.name)}</span>
          <span class="mono">${escapeHtml(String(entry.count))}x</span>
        </div>
      `,
    )
    .join("");
}

function renderOverviewIndirectControls(rows) {
  if (!rows.length) {
    return "";
  }

  return rows
    .map((entry) => {
      const resolvedTarget =
        entry.resolved_target === null || entry.resolved_target === undefined
          ? `<span class="mono">n/a</span>`
          : renderJumpButton(entry.resolved_target);
      return `
        <div class="analysis-row">
          <span class="mono">Indirect ${escapeHtml(entry.kind)}</span>
          ${renderJumpButton(entry.first_instruction_address)}
          <span class="mono">${escapeHtml(entry.via)}</span>
          ${resolvedTarget}
          <span class="mono">${escapeHtml(String(entry.count))}x</span>
        </div>
      `;
    })
    .join("");
}

function renderOverviewJumpTables(rows) {
  if (!rows.length) {
    return "";
  }

  return rows
    .map(
      (entry) => `
        <div class="analysis-row">
          <span class="mono">Jump Table</span>
          ${renderJumpButton(entry.first_instruction_address)}
          ${renderJumpButton(entry.table_base)}
          <span class="mono">${escapeHtml(entry.index_register)}</span>
          <span class="mono">${escapeHtml(String(entry.element_size))}B</span>
          <span class="mono">${escapeHtml(String(entry.count))}x</span>
        </div>
      `,
    )
    .join("");
}

function renderOverviewCacheLinks(rows) {
  if (!rows.length) {
    return "";
  }

  return rows
    .map(
      (entry) => `
        <div class="analysis-row">
          <span class="mono">Cache Link</span>
          ${renderJumpButton(entry.first_instruction_address)}
          <span class="mono">${escapeHtml(entry.symbol_name)}</span>
          <span class="mono">${escapeHtml(entry.provider_kind)}</span>
          <span class="mono">${escapeHtml(entry.provider_install_name)}</span>
          <span class="mono">${escapeHtml(String(entry.count))}x</span>
        </div>
      `,
    )
    .join("");
}

function renderOverviewObjc(rows) {
  if (!rows.length) {
    return "";
  }

  return rows
    .map(
      (entry) => `
        <div class="analysis-row">
          <span class="mono">ObjC ${escapeHtml(entry.kind)}</span>
          ${renderJumpButton(entry.first_instruction_address)}
          ${renderJumpButton(entry.target_address)}
          <span class="mono">${escapeHtml(entry.name ?? "-")}</span>
          <span class="mono">${escapeHtml(String(entry.count))}x</span>
        </div>
      `,
    )
    .join("");
}

function renderOverviewValues(rows) {
  if (!rows.length) {
    return "";
  }

  return rows
    .map(
      (entry) => `
        <div class="analysis-row">
          <span class="mono">${escapeHtml(entry.register)}</span>
          ${renderJumpButton(entry.first_instruction_address)}
          <span class="mono">${escapeHtml(formatAddress(entry.value))}</span>
          <span class="mono">${escapeHtml(entry.kind)}</span>
          <span class="mono">${escapeHtml(entry.source)}</span>
          <span class="mono">${escapeHtml(String(entry.count))}x</span>
        </div>
      `,
    )
    .join("");
}

function renderDetailBlock(title, rows) {
  return `
    <section class="inspector-block">
      <h3>${title}</h3>
      <div class="detail-list">
        ${renderDetailRows(rows)}
      </div>
    </section>
  `;
}

function renderDetailRows(rows) {
  return rows
    .map(
      ([key, value]) => `
        <div class="detail-row">
          <span class="detail-key">${escapeHtml(key)}</span>
          <span class="mono">${escapeHtml(String(value))}</span>
        </div>
      `,
    )
    .join("");
}

function renderAnalysisBlocks(blocks, blockById = buildBlockMap(blocks)) {
  if (!blocks.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Basic Blocks</div>
      ${blocks
        .map(
          (block) => `
            <div class="analysis-row">
              <span class="mono">#${escapeHtml(block.id)}</span>
              ${renderJumpButton(block.start_address)}
              <span class="mono">${escapeHtml(formatAddress(block.end_address))}</span>
              <span class="mono">${escapeHtml(String(block.instruction_count))} insn</span>
              <span class="detail-key">pred</span>
              ${renderBlockLinks(block.predecessors || [], blockById)}
              <span class="detail-key">succ</span>
              ${renderBlockLinks(block.successors || [], blockById)}
              ${renderExternalSuccessors(block.external_successors || [])}
              ${block.terminal ? '<span class="mono">return</span>' : ""}
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function buildBlockMap(blocks) {
  return new Map(blocks.map((block) => [String(block.id), block]));
}

function renderBlockLinks(blockIds, blockById) {
  if (!blockIds.length) {
    return '<span class="mono">-</span>';
  }

  return `
    <span class="block-link-list">
      ${blockIds
        .map((id) => renderBlockLink(id, blockById))
        .join("")}
    </span>
  `;
}

function renderBlockLink(id, blockById) {
  if (id === null || id === undefined) {
    return '<span class="mono">-</span>';
  }

  const label = `#${id}`;
  const block = blockById.get(String(id));
  if (!block) {
    return `<span class="mono">${escapeHtml(label)}</span>`;
  }

  const target = formatAddress(block.start_address);
  if (target === "n/a") {
    return `<span class="mono">${escapeHtml(label)}</span>`;
  }

  return `
    <button
      class="jump-button block-jump mono"
      type="button"
      data-jump-target="${escapeHtml(target)}"
      title="${escapeHtml(`${label} starts at ${target}`)}"
      aria-label="${escapeHtml(`Jump to block ${label} at ${target}`)}"
    >
      ${escapeHtml(label)}
    </button>
  `;
}

function renderExternalSuccessors(externalSuccessors) {
  if (!externalSuccessors.length) {
    return "";
  }

  return `
    <span class="detail-key">external</span>
    <span class="external-successor-list">
      ${externalSuccessors
        .map(
          (successor) => `
            <span class="external-successor">
              <span class="mono">${escapeHtml(successor.kind ?? "edge")}</span>
              ${renderJumpButton(successor.target_address)}
              ${
                successor.via
                  ? `<span class="mono">via ${escapeHtml(successor.via)}</span>`
                  : ""
              }
            </span>
          `,
        )
        .join("")}
    </span>
  `;
}

function renderAnalysisEdges(edges, blockById = new Map()) {
  if (!edges.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Analysis Edges</div>
      ${edges
        .map(
          (edge) => `
            <div class="analysis-row">
              <span class="mono">${escapeHtml(edge.kind)}</span>
              ${renderBlockLink(edge.from_block, blockById)}
              ${renderJumpButton(edge.source_address)}
              <span class="detail-key">to</span>
              ${
                edge.target_address === null
                  ? '<span class="mono">return</span>'
                  : renderJumpButton(edge.target_address)
              }
              ${renderBlockLink(edge.to_block, blockById)}
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderAnalysisTargetUses(label, rows) {
  if (!rows.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">${escapeHtml(label)}</div>
      ${rows
        .map(
          (entry) => `
            <div class="analysis-row">
              <span class="detail-key">at</span>
              ${renderJumpButton(entry.instruction_address)}
              <span class="detail-key">target</span>
              ${renderJumpButton(entry.target_address)}
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderAnalysisIndirectControls(rows) {
  if (!rows.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Indirect Controls</div>
      ${rows
        .map((entry) => {
          const resolvedTarget =
            entry.resolved_target === null || entry.resolved_target === undefined
              ? '<span class="mono">unresolved</span>'
              : renderJumpButton(entry.resolved_target);
          return `
            <div class="analysis-row">
              <span class="mono">${escapeHtml(entry.kind)}</span>
              ${renderJumpButton(entry.instruction_address)}
              <span class="detail-key">via</span>
              <span class="mono">${escapeHtml(entry.via)}</span>
              <span class="detail-key">target</span>
              ${resolvedTarget}
            </div>
          `;
        })
        .join("")}
    </div>
  `;
}

function renderAnalysisJumpTables(rows) {
  if (!rows.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Jump Tables</div>
      ${rows
        .map(
          (entry) => `
            <div class="analysis-row">
              ${renderJumpButton(entry.instruction_address)}
              <span class="detail-key">base</span>
              ${renderJumpButton(entry.table_base)}
              <span class="detail-key">index</span>
              <span class="mono">${escapeHtml(entry.index_register)}</span>
              <span class="detail-key">elem</span>
              <span class="mono">${escapeHtml(String(entry.element_size))}B</span>
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderAnalysisRecoveredValues(rows) {
  if (!rows.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Recovered Values</div>
      ${rows
        .map(
          (entry) => `
            <div class="analysis-row">
              ${renderJumpButton(entry.instruction_address)}
              <span class="mono">${escapeHtml(entry.register)}</span>
              <span class="mono">${escapeHtml(formatAddress(entry.value))}</span>
              <span class="mono">${escapeHtml(entry.kind)}</span>
              <span class="mono">${escapeHtml(entry.source)}</span>
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderAnalysisImports(imports, cacheLinks) {
  if (!imports.length && !cacheLinks.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Imports</div>
      ${imports
        .map(
          (entry) => `
            <div class="analysis-row">
              ${renderJumpButton(entry.instruction_address)}
              <span class="mono">${escapeHtml(entry.dylib)}</span>
              <span class="mono">${escapeHtml(entry.name)}</span>
            </div>
          `,
        )
        .join("")}
      ${cacheLinks
        .map(
          (entry) => `
            <div class="analysis-row">
              ${renderJumpButton(entry.instruction_address)}
              <span class="mono">${escapeHtml(entry.provider_kind)}</span>
              <span class="mono">${escapeHtml(entry.provider_install_name)}</span>
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderAnalysisObjcReferences(references) {
  if (!references.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">ObjC References</div>
      ${references
        .map((entry) => {
          const resolved =
            entry.resolved_address === null
              ? '<span class="mono">-</span>'
              : renderJumpButton(entry.resolved_address);
          const importProvider =
            entry.import_dylib || entry.import_symbol
              ? `${entry.import_dylib ?? "-"}:${entry.import_symbol ?? "-"}`
              : "-";
          return `
            <div class="analysis-row">
              ${renderJumpButton(entry.instruction_address)}
              <span class="mono">${escapeHtml(entry.kind)}</span>
              ${renderJumpButton(entry.target_address)}
              ${resolved}
              <span class="mono">${escapeHtml(entry.name ?? "-")}</span>
              <span class="mono">${escapeHtml(importProvider)}</span>
            </div>
          `;
        })
        .join("")}
    </div>
  `;
}

function renderJumpButton(value) {
  const label = formatAddress(value);
  if (label === "n/a") {
    return '<span class="mono">n/a</span>';
  }

  return `
    <button class="jump-button mono" type="button" data-jump-target="${escapeHtml(label)}">
      ${escapeHtml(label)}
    </button>
  `;
}

function renderReferenceBlock(references) {
  if (!references.length) {
    return renderEmptyBlock("References", "No references recorded for this instruction.");
  }

  return `
    <section class="inspector-block">
      <h3>References</h3>
      <div class="data-list">
        ${references
          .map((reference) => {
            const details = Object.entries(reference)
              .filter(([key]) => key !== "type")
              .map(([key, value]) => {
                const addressLabel = formatAddress(value);
                if (
                  (key === "target" || key.endsWith("_address")) &&
                  addressLabel !== "n/a"
                ) {
                  return `
                    <div class="detail-row">
                      <span class="detail-key">${escapeHtml(key)}</span>
                      ${renderJumpButton(value)}
                    </div>
                  `;
                }

                return `
                  <div class="detail-row">
                    <span class="detail-key">${escapeHtml(key)}</span>
                    <span class="mono">${escapeHtml(formatValue(value))}</span>
                  </div>
                `;
              })
              .join("");

            return `
              <div class="data-item">
                <div class="item-eyebrow">${escapeHtml(reference.type || "reference")}</div>
                <div class="detail-list">${details}</div>
              </div>
            `;
          })
          .join("")}
      </div>
    </section>
  `;
}

function renderObjectBlock(title, entries) {
  if (!entries.length) {
    return renderEmptyBlock(title, `No ${title.toLowerCase()} for the selected instruction.`);
  }

  return `
    <section class="inspector-block">
      <h3>${escapeHtml(title)}</h3>
      <div class="data-list">
        ${entries
          .map(
            (entry) => `
              <div class="data-item">
                ${Object.entries(entry)
                  .map(
                    ([key, value]) => `
                      <div class="detail-row">
                        <span class="detail-key">${escapeHtml(key)}</span>
                        <span class="mono">${escapeHtml(formatValue(value))}</span>
                      </div>
                    `,
                  )
                  .join("")}
              </div>
            `,
          )
          .join("")}
      </div>
    </section>
  `;
}

function renderEmptyBlock(title, message) {
  return `
    <section class="inspector-block">
      <h3>${escapeHtml(title)}</h3>
      <p class="inspector-empty">${escapeHtml(message)}</p>
    </section>
  `;
}

async function jumpToAddress(target) {
  const label = formatAddress(target);
  const targetBigInt = addressBigInt(label);
  if (targetBigInt === null) {
    setBanner("Cannot jump to an invalid address.");
    return;
  }

  document.querySelector('input[name="target-kind"][value="address"]').checked = true;
  updateTargetControls();
  elements.addressInput.value = label;
  elements.fromInput.value = "";
  const to = parsedIntegerValue(elements.toInput);
  if (to !== null && to <= targetBigInt) {
    elements.toInput.value = "";
  }
  await runDisasm();
}

function formatValue(value) {
  if (value === null || value === undefined) {
    return "null";
  }

  if (typeof value === "number") {
    return Number.isInteger(value) ? formatAddress(value) : String(value);
  }

  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }

  if (Array.isArray(value)) {
    return `[${value.map(formatValue).join(", ")}]`;
  }

  if (typeof value === "object") {
    return JSON.stringify(value);
  }

  return String(value);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function installUploadHandlers() {
  elements.fileInput.addEventListener("change", async (event) => {
    const [file] = event.target.files || [];
    if (file) {
      await loadImage(file);
    }
  });

  ["dragenter", "dragover"].forEach((eventName) => {
    elements.dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      elements.dropzone.classList.add("drag-over");
    });
  });

  ["dragleave", "drop"].forEach((eventName) => {
    elements.dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      elements.dropzone.classList.remove("drag-over");
    });
  });

  elements.dropzone.addEventListener("drop", async (event) => {
    const [file] = event.dataTransfer?.files || [];
    if (file) {
      await loadImage(file);
    }
  });
}

function installEventHandlers() {
  installUploadHandlers();
  document.querySelectorAll('input[name="target-kind"]').forEach((input) => {
    input.addEventListener("change", updateTargetControls);
  });
  elements.runButton.addEventListener("click", runDisasm);
  elements.symbolInput.addEventListener("input", () => {
    clearTimeout(state.symbolLookupTimer);
    state.symbolLookupTimer = window.setTimeout(fetchSymbolSuggestions, 180);
  });
}

updateTargetControls();
updateFileSummary();
renderDisasm();
renderInspector();
syncWorkbenchState();
installEventHandlers();
