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
