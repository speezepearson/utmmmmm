import { useEffect, useRef } from "react";
import cytoscape from "cytoscape";
// @ts-expect-error no type declarations for cytoscape-elk
import elk from "cytoscape-elk";
import type { GraphSpec } from "./buildGraph";
import type { State, Symbol } from "./types";

cytoscape.use(elk);

type Props = {
  graph: GraphSpec;
  currentState: State;
  currentSymbol?: Symbol;
};

export function TMStateGraph({ graph, currentState, currentSymbol }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<cytoscape.Core | null>(null);

  // Build cytoscape elements + run layout
  useEffect(() => {
    if (!containerRef.current) return;

    const elements: cytoscape.ElementDefinition[] = [];

    for (const cluster of graph.clusters) {
      elements.push({
        data: {
          id: `cluster-${cluster.id}`,
          label: cluster.label,
          parent: cluster.parent ? `cluster-${cluster.parent}` : undefined,
        },
        classes: "cluster",
      });
    }

    for (const node of graph.nodes) {
      elements.push({
        data: {
          id: node.id,
          label: node.label,
          parent: node.cluster ? `cluster-${node.cluster}` : undefined,
        },
      });
    }

    for (const edge of graph.edges) {
      elements.push({
        data: {
          id: edge.id,
          source: edge.source,
          target: edge.target,
          label: edge.label,
          symbols: edge.symbols,
        },
        classes: edge.source === edge.target ? "selfloop" : undefined,
      });
    }

    const cy = cytoscape({
      container: containerRef.current,
      elements,
      style: [
        {
          selector: "node",
          style: {
            label: "data(label)",
            "text-valign": "center",
            "text-halign": "center",
            "font-size": "10px",
            "font-family": "ui-monospace, Consolas, monospace",
            width: "label",
            height: "label",
            shape: "roundrectangle",
            "background-color": "#e2e8f0",
            "border-width": 1,
            "border-color": "#94a3b8",
            color: "#1e293b",
            "text-wrap": "wrap",
            "text-max-width": "15em",
            padding: "6px",
          },
        },
        {
          selector: "node.cluster",
          style: {
            "text-valign": "top",
            "text-halign": "center",
            "font-size": "11px",
            "font-weight": "bold",
            "background-opacity": 0.08,
            "border-width": 1,
            "border-color": "#cbd5e1",
            "border-opacity": 0.5,
            padding: "16px",
            shape: "roundrectangle",
            color: "#475569",
            "text-wrap": "none",
            width: undefined as unknown as number,
            height: undefined as unknown as number,
          },
        },
        {
          selector: "node.active-state",
          style: {
            "background-color": "#6366f1",
            "border-color": "#4338ca",
            "border-width": 3,
            color: "#ffffff",
            "font-weight": "bold",
            "z-index": 999,
          },
        },
        {
          selector: "edge",
          style: {
            width: 1,
            "line-color": "#cbd5e1",
            "target-arrow-color": "#cbd5e1",
            "target-arrow-shape": "triangle",
            "curve-style": "bezier",
            label: "data(label)",
            "font-size": "7px",
            "font-family": "ui-monospace, Consolas, monospace",
            "text-rotation": "autorotate",
            color: "#64748b",
            "text-background-color": "#ffffff",
            "text-background-opacity": 0.8,
            "text-background-padding": "2px",
            "arrow-scale": 0.6,
          },
        },
        {
          selector: "edge.selfloop",
          style: {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            "curve-style": "loop" as any,
            "loop-direction": "0deg",
            "loop-sweep": "-90deg",
          },
        },
        {
          selector: "edge.active-edge",
          style: {
            width: 3,
            "line-color": "#ef4444",
            "target-arrow-color": "#ef4444",
            "font-weight": "bold",
            color: "#ef4444",
            "z-index": 999,
          },
        },
      ],
      layout: {
        name: "elk",
        nodeDimensionsIncludeLabels: true,
        elk: {
          algorithm: "layered",
          "elk.direction": "RIGHT",
          "spacing.nodeNode": "15",
          "layered.spacing.nodeNodeBetweenLayers": "30",
          "spacing.edgeEdge": "8",
          "spacing.edgeNode": "10",
          "layered.compaction.postCompaction.strategy": "IMPROVE_STRAIGHTNESS",
          hierarchyHandling: "INCLUDE_CHILDREN",
        },
        padding: 20,
      } as cytoscape.LayoutOptions,
      minZoom: 0.05,
      maxZoom: 5,
      wheelSensitivity: 0.3,
    });

    cyRef.current = cy;

    return () => {
      cy.destroy();
      cyRef.current = null;
    };
  }, [graph]);

  // Update highlighting
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;

    cy.elements(".active-state").removeClass("active-state");
    cy.elements(".active-edge").removeClass("active-edge");

    const stateNode = cy.getElementById(String(currentState));
    if (stateNode.length) {
      stateNode.addClass("active-state");
    }

    if (currentSymbol !== undefined) {
      const sym = String(currentSymbol);
      const st = String(currentState);
      cy.edges().forEach((edge) => {
        const syms: string[] = edge.data("symbols") ?? [];
        if (edge.data("source") === st && syms.includes(sym)) {
          edge.addClass("active-edge");
        }
      });
    }
  }, [currentState, currentSymbol]);

  return (
    <div
      ref={containerRef}
      className="tm-state-graph"
      style={{
        width: "100%",
        height: "500px",
        border: "1px solid var(--border, #ccc)",
        borderRadius: "8px",
        margin: "8px 0",
        background: "var(--code-bg, #f8fafc)",
      }}
    />
  );
}
