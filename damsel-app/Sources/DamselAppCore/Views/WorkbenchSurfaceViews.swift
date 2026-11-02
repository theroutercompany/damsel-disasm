import SwiftUI

/// The pseudocode surface (plan feature 20b): decompiled lines with a
/// source-address gutter and an unknown-count badge.
struct PseudocodePane: View {
    @ObservedObject var store: WorkbenchStore

    var body: some View {
        VStack(spacing: 0) {
            surfaceHeader
            Divider()
            content
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var surfaceHeader: some View {
        HStack(spacing: 10) {
            Button {
                Task { await store.loadPseudocode() }
            } label: {
                Label("Decompile", systemImage: "play.fill")
            }
            .disabled(store.loadedImage == nil)

            if let pseudocode = store.pseudocode {
                Text(pseudocode.function)
                    .font(.system(.headline, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                Spacer()
                Text("\(pseudocode.lines.count) lines")
                    .foregroundStyle(.secondary)
                if pseudocode.unknownCount > 0 {
                    Text("\(pseudocode.unknownCount) unknown")
                        .font(.caption.weight(.semibold))
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.orange.opacity(0.25), in: Capsule())
                } else {
                    Text("clean")
                        .font(.caption.weight(.semibold))
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.green.opacity(0.2), in: Capsule())
                }
            } else {
                Spacer()
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var content: some View {
        if let pseudocode = store.pseudocode {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(pseudocode.lines.enumerated()), id: \.offset) { _, line in
                        HStack(alignment: .top, spacing: 12) {
                            Text(line.addressLabel ?? "")
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(.secondary)
                                .frame(width: 90, alignment: .trailing)
                            Text(line.text)
                                .font(.system(size: 12, design: .monospaced))
                                .textSelection(.enabled)
                        }
                    }
                }
                .padding(14)
            }
        } else {
            SurfaceEmptyState(
                systemImage: "curlybraces.square",
                title: "No pseudocode loaded",
                message: "Set a symbol or address target and press Decompile."
            )
        }
    }
}

/// The graph surface (plan feature 20c): the control-flow canvas —
/// blocks laid out by longest-path layering, edges drawn as arrows
/// (taken solid, fallthrough dashed).
struct CFGCanvasView: View {
    @ObservedObject var store: WorkbenchStore

    var body: some View {
        VStack(spacing: 0) {
            surfaceHeader
            Divider()
            content
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var surfaceHeader: some View {
        HStack(spacing: 10) {
            Button {
                Task { await store.loadCfg() }
            } label: {
                Label("Build Graph", systemImage: "point.3.connected.trianglepath.dotted")
            }
            .disabled(store.loadedImage == nil)

            if let cfg = store.cfg {
                Text(cfg.function)
                    .font(.system(.headline, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                Spacer()
                Text("\(cfg.blocks.count) blocks · \(cfg.edges.count) edges")
                    .foregroundStyle(.secondary)
            } else {
                Spacer()
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var content: some View {
        if let cfg = store.cfg {
            let placements = CfgLayout.placements(for: cfg)
            let size = CfgLayout.canvasSize(for: placements)
            let placementByAddress = Dictionary(
                uniqueKeysWithValues: placements.map { ($0.blockAddress, $0) }
            )
            ScrollView([.horizontal, .vertical]) {
                ZStack {
                    Canvas { context, _ in
                        // Edges under blocks.
                        for edge in cfg.edges {
                            guard
                                let from = placementByAddress[edge.from],
                                let to = placementByAddress[edge.to]
                            else { continue }
                            drawEdge(&context, from: from, to: to, kind: edge.kind)
                        }
                    }
                    .frame(width: size.width, height: size.height)

                    ForEach(placements, id: \.blockAddress) { placement in
                        if let block = cfg.blocks.first(where: { $0.address == placement.blockAddress }) {
                            blockView(block)
                                .position(
                                    x: CfgLayout.origin(for: placement).x + CfgLayout.blockWidth / 2,
                                    y: CfgLayout.origin(for: placement).y + CfgLayout.blockHeight / 2
                                )
                        }
                    }
                }
                .frame(width: size.width, height: size.height)
                .padding(8)
            }
        } else {
            SurfaceEmptyState(
                systemImage: "point.3.connected.trianglepath.dotted",
                title: "No graph loaded",
                message: "Set a symbol or address target and press Build Graph."
            )
        }
    }

    private func blockView(_ block: CfgBlock) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(block.address)
                .font(.system(size: 10, design: .monospaced).weight(.semibold))
            Text("\(block.statements) stmts")
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(.secondary)
            Text(block.terminator)
                .font(.system(size: 9, design: .monospaced))
                .padding(.horizontal, 5)
                .padding(.vertical, 1)
                .background(terminatorColor(block.terminator).opacity(0.2), in: Capsule())
        }
        .frame(width: CfgLayout.blockWidth, height: CfgLayout.blockHeight, alignment: .topLeading)
        .padding(8)
        .background(Color(nsColor: .controlBackgroundColor))
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .strokeBorder(terminatorColor(block.terminator), lineWidth: 1.2)
        )
        .clipShape(RoundedRectangle(cornerRadius: 6))
    }

    private func terminatorColor(_ kind: String) -> Color {
        switch kind {
        case "return": .green
        case "cond_branch": .purple
        case "branch": .blue
        case "trap": .red
        default: .gray
        }
    }

    private func drawEdge(
        _ context: inout GraphicsContext,
        from: CfgLayout.Placement,
        to: CfgLayout.Placement,
        kind: String
    ) {
        let fromPoint = CGPoint(
            x: CfgLayout.origin(for: from).x + CfgLayout.blockWidth / 2,
            y: CfgLayout.origin(for: from).y + CfgLayout.blockHeight
        )
        let toPoint = CGPoint(
            x: CfgLayout.origin(for: to).x + CfgLayout.blockWidth / 2,
            y: CfgLayout.origin(for: to).y
        )
        var path = Path()
        let mid = (fromPoint.y + toPoint.y) / 2
        path.move(to: fromPoint)
        path.addCurve(
            to: toPoint,
            control1: CGPoint(x: fromPoint.x, y: mid),
            control2: CGPoint(x: toPoint.x, y: mid)
        )
        let color: Color = kind == "taken" ? .purple : (kind == "fallthrough" ? .blue : .gray)
        let style = StrokeStyle(lineWidth: 1.4, dash: kind == "fallthrough" ? [4, 3] : [])
        context.stroke(path, with: .color(color.opacity(0.7)), style: style)
        // Arrowhead.
        var arrow = Path()
        arrow.addLines([
            CGPoint(x: toPoint.x - 4, y: toPoint.y + 6),
            CGPoint(x: toPoint.x, y: toPoint.y),
            CGPoint(x: toPoint.x + 4, y: toPoint.y + 6),
        ])
        context.stroke(arrow, with: .color(color.opacity(0.7)), lineWidth: 1.4)
    }
}

/// The hex surface (plan feature 20b): address + length controls over
/// 16-byte rows (address, hex pairs, ascii).
struct HexView: View {
    @ObservedObject var store: WorkbenchStore

    var body: some View {
        VStack(spacing: 0) {
            surfaceHeader
            Divider()
            content
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var surfaceHeader: some View {
        HStack(spacing: 10) {
            TextField("Address (0x…)", text: $store.hexAddressText)
                .font(.system(size: 12, design: .monospaced))
                .frame(width: 160)
                .onSubmit { Task { await store.loadHex() } }
            TextField("Length", text: $store.hexLengthText)
                .font(.system(size: 12, design: .monospaced))
                .frame(width: 70)
                .onSubmit { Task { await store.loadHex() } }
            Button {
                Task { await store.loadHex() }
            } label: {
                Label("Read", systemImage: "arrow.down.doc")
            }
            .disabled(store.loadedImage == nil)
            if let hexData = store.hexData {
                Spacer()
                Text("\(hexData.section) · \(hexData.length) bytes")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.secondary)
            } else {
                Spacer()
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var content: some View {
        if let hexData = store.hexData {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 1) {
                    ForEach(hexData.rows) { row in
                        HStack(alignment: .top, spacing: 14) {
                            Text(row.address)
                                .foregroundStyle(.secondary)
                                .frame(width: 90, alignment: .trailing)
                            Text(row.hexText)
                                .frame(width: 330, alignment: .leading)
                            Text(row.ascii)
                                .foregroundStyle(.secondary)
                        }
                        .font(.system(size: 11, design: .monospaced))
                        .textSelection(.enabled)
                        .padding(.vertical, 1)
                    }
                }
                .padding(14)
            }
        } else {
            SurfaceEmptyState(
                systemImage: "square.grid.3x3.middle.left",
                title: "No bytes loaded",
                message: "Enter an address (or leave blank for the text section) and press Read."
            )
        }
    }
}

/// Shared empty state for the surfaces.
struct SurfaceEmptyState: View {
    let systemImage: String
    let title: String
    let message: String

    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: systemImage)
                .font(.system(size: 34, weight: .light))
                .foregroundStyle(.secondary)
            Text(title)
                .font(.headline)
            Text(message)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
