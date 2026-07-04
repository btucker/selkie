import importlib.util
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "generate_corpus", Path(__file__).parent / "generate-corpus.py"
)
gc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gc)


def test_registry_has_flowchart():
    assert "flowchart" in gc.EMITTERS


def test_filename_format():
    c = gc.Case(family="shapes", name="diamond", source="flowchart TD\n  A{q}\n")
    assert gc.filename("flowchart", c) == "gen_flowchart_shapes_diamond.mmd"


def test_write_is_idempotent(tmp_path):
    cases = [gc.Case("shapes", "sq", "flowchart TD\n  A[x]\n")]
    first = gc.write_cases("flowchart", cases, tmp_path)
    contents1 = {p.name: p.read_text() for p in tmp_path.glob("*.mmd")}
    second = gc.write_cases("flowchart", cases, tmp_path)
    contents2 = {p.name: p.read_text() for p in tmp_path.glob("*.mmd")}
    assert contents1 == contents2
    assert [p.name for p in first] == [p.name for p in second]


def test_write_clobbers_only_own_namespace(tmp_path):
    (tmp_path / "channel_flowchart_keep.mmd").write_text("flowchart TD\n  K[keep]\n")
    (tmp_path / "gen_flowchart_shapes_stale.mmd").write_text("stale")
    gc.write_cases("flowchart", [gc.Case("shapes", "sq", "flowchart TD\n  A[x]\n")], tmp_path)
    assert (tmp_path / "channel_flowchart_keep.mmd").exists()
    assert not (tmp_path / "gen_flowchart_shapes_stale.mmd").exists()
    assert (tmp_path / "gen_flowchart_shapes_sq.mmd").exists()


def test_no_duplicate_filenames():
    cases = gc.EMITTERS["flowchart"]()
    names = [gc.filename("flowchart", c) for c in cases]
    assert len(names) == len(set(names)), "duplicate generated filenames"


def test_flowchart_families_present():
    cases = gc.EMITTERS["flowchart"]()
    families = {c.family for c in cases}
    for required in ("shapes", "directions", "edges", "labels"):
        assert required in families, f"missing family {required}"


def test_all_mermaid_supported_shapes_covered():
    cases = gc.EMITTERS["flowchart"]()
    shape_cases = [c for c in cases if c.family == "shapes"]
    # 14 mermaid-renderable shapes + 1 grid (ellipse A(-t-) excluded: selkie
    # parses it but mermaid v11 rejects it, so there is no reference).
    assert len(shape_cases) >= 15
    assert not any(c.name == "ellipse" for c in shape_cases)


def test_all_5_directions_covered():
    cases = gc.EMITTERS["flowchart"]()
    dir_names = {c.name for c in cases if c.family == "directions"}
    assert dir_names == {"tb", "td", "lr", "rl", "bt"}


def test_every_case_starts_with_flowchart_keyword():
    for c in gc.EMITTERS["flowchart"]():
        assert c.source.lstrip().startswith(("flowchart", "graph", "%%{")), c.name


def test_all_families_present():
    cases = gc.EMITTERS["flowchart"]()
    families = {c.family for c in cases}
    expected = {
        "shapes", "directions", "edges", "labels",
        "subgraphs", "styling", "interactions", "a11y_directives",
        "integration", "stretch",
    }
    assert expected <= families, expected - families


def test_corpus_size_in_target_range():
    cases = gc.EMITTERS["flowchart"]()
    assert 60 <= len(cases) <= 100, len(cases)


def test_styling_family_uses_classdef_and_linkstyle():
    cases = gc.EMITTERS["flowchart"]()
    styling = "\n".join(c.source for c in cases if c.family == "styling")
    assert "classDef" in styling
    assert "linkStyle" in styling
    assert "\n  style " in "\n" + styling


def test_isolated_and_connected_subgraph_cases_exist():
    names = {c.name for c in gc.EMITTERS["flowchart"]() if c.family == "subgraphs"}
    assert "isolated" in names
    assert "connected" in names
    assert "own_direction" in names
