import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT_PATH = REPO_ROOT / "scripts" / "generate-specs.py"


def load_generator():
    spec = importlib.util.spec_from_file_location("generate_specs", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class GenerateSpecsTests(unittest.TestCase):
    def test_extracts_rust_doc_comment_specs(self):
        generator = load_generator()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "tests" / "flowchart_rendering.rs"
            source.parent.mkdir()
            source.write_text(
                '/// @spec FLOW-1.1: When a flowchart declares a named subgraph, Selkie shall preserve the subgraph title text.\n'
                "#[test]\n"
                "fn subgraph_title() {}\n",
                encoding="utf-8",
            )

            specs = generator.collect_specs(root)

        self.assertEqual(len(specs), 1)
        self.assertEqual(specs[0].spec_id, "FLOW-1.1")
        self.assertEqual(
            specs[0].text,
            "When a flowchart declares a named subgraph, Selkie shall preserve the subgraph title text.",
        )
        self.assertEqual(specs[0].path, "tests/flowchart_rendering.rs")
        self.assertEqual(specs[0].line, 1)

    def test_ignores_spec_text_inside_string_literals(self):
        generator = load_generator()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "tests" / "generator_tests.py"
            source.parent.mkdir()
            source.write_text(
                'fixture = "/// @spec FLOW-9.9: When text appears in a fixture, the application shall ignore it."\n',
                encoding="utf-8",
            )

            specs = generator.collect_specs(root)

        self.assertEqual(specs, [])

    def test_render_groups_specs_by_prefix(self):
        generator = load_generator()
        specs = [
            generator.Spec(
                spec_id="FLOW-2.1",
                text="When a flowchart edge has a label, Selkie shall preserve the label text.",
                path="tests/flowchart.rs",
                line=4,
            ),
            generator.Spec(
                spec_id="EVAL-1.1",
                text="When specs are generated, the application shall group them by prefix.",
                path="tests/eval.rs",
                line=8,
            ),
        ]

        rendered = generator.render_specs(specs)

        self.assertIn("## EVAL", rendered)
        self.assertIn("## FLOW", rendered)
        self.assertIn("### FLOW-2.1", rendered)
        self.assertIn("`tests/flowchart.rs:4`", rendered)

    def test_duplicate_spec_ids_fail_validation(self):
        generator = load_generator()
        specs = [
            generator.Spec("FLOW-1.1", "First text.", "a.rs", 1),
            generator.Spec("FLOW-1.1", "Second text.", "b.rs", 2),
        ]

        with self.assertRaisesRegex(generator.SpecError, "Duplicate @spec ID FLOW-1.1"):
            generator.validate_specs(specs)

    def test_check_mode_fails_when_specs_file_is_stale(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "tests" / "flowchart_rendering.rs"
            source.parent.mkdir()
            source.write_text(
                "/// @spec FLOW-1.1: When a flowchart declares a named subgraph, Selkie shall preserve the subgraph title text.\n",
                encoding="utf-8",
            )
            (root / "SPECS.md").write_text("# stale\n", encoding="utf-8")

            result = subprocess.run(
                [sys.executable, str(SCRIPT_PATH), "--root", str(root), "--check"],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SPECS.md is stale", result.stderr)


if __name__ == "__main__":
    unittest.main()
