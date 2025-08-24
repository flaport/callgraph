"""Example usage of the callgraph library with NetworkX and matplotlib visualization.

This module provides utility functions to create NetworkX graphs from call graph data
and plot them using matplotlib.
"""

from __future__ import annotations

from pathlib import Path

import callgraph
import matplotlib.pyplot as plt
import networkx as nx


def create_callgraph(
    lib_paths: list[Path],
    prefix: str | None = None,
) -> nx.DiGraph:
    """Create a directed graph from the call graph data."""
    result = callgraph.generate_call_graph(
        lib_paths=[str(p) for p in lib_paths],
        prefix=prefix,
    )

    # Create directed graph
    G = nx.DiGraph()  # noqa: N806

    functions = result.get("functions", {})

    # Add nodes for all functions
    for resolved_name, func_info in functions.items():
        func_info["resolved_name"] = resolved_name
        if not _is_valid_node(func_info):
            continue

        G.add_node(
            _name(func_info),
            module=func_info.get("module", ""),
            line=func_info.get("line", 0),
            type_=_type(func_info),
        )

    for func_info in functions.values():
        if not _is_valid_node(func_info):
            continue

        for called_func in func_info.get("resolved_calls", []):
            if not _is_valid_node(functions.get(called_func, {})):
                continue
            G.add_edge(_name(func_info), _name(functions[called_func]))

        for called_func in func_info.get("resolved_component_gets", []):
            if not _is_valid_node(functions.get(called_func, {})):
                continue
            G.add_edge(_name(func_info), _name(functions[called_func]))

    return G


def _is_valid_node(func_info: dict) -> bool:
    decorators = func_info.get("decorators", [])
    resolved_decorators = func_info.get("resolved_decorators", [])
    allowed_decorators = [
        "yaml",
        "gdsfactory._cell.cell",
        "gdsfactory._cell.cell_with_module_name",
    ]
    if not any(
        (k in resolved_decorators or k in decorators) for k in allowed_decorators
    ):
        return False
    if not _name(func_info):
        return False
    if func_info.get("module", "").startswith("gdsfactory."):  # noqa: SIM103
        return False
    return True


def _name(func_info: dict) -> str:
    """Get the name of the function or component."""
    return func_info.get("name", "")


def _type(func_info: dict) -> str:
    return func_info["resolved_name"].split(".")[0]


if __name__ == "__main__":
    cwd = Path(__file__).parent
    G = create_callgraph(
        lib_paths=[cwd / "mycspdk", cwd / "cspdk", cwd / "gdsfactory"],
        prefix="cspdk.si220.cband",
    )
    color_map = {"gdsfactory": "C0", "cspdk": "C1", "mycspdk": "C2"}
    node_colors = [color_map[G.nodes[n]["type_"]] for n in G.nodes()]
    pos = nx.nx_pydot.graphviz_layout(G, prog="neato")
    pos = {k: (-y, x) for k, (x, y) in pos.items()}
    # pos = nx.kamada_kawai_layout(G)

    nx.draw(
        G,
        pos,
        with_labels=True,
        arrows=True,
        node_size=100,
        node_color=node_colors,
        font_size=6,
    )

    for k, c in color_map.items():
        plt.scatter([], [], color=c, label=k)

    plt.legend(loc="upper left")
    plt.show()
