from jorm.parser import parse_dag_text, validate_dag

def test_parse_txt():
    parsed = parse_dag_text("tests/fixtures/simple.txt")
    assert parsed["name"] == "simple"
    assert "extract_sales" in parsed["tasks"]

def test_validate_cycles():
    parsed = {
        "name": "cycle",
        "schedule": "every 10 minutes",
        "tasks": ["a", "b"],
        "dependencies": [("a", "b"), ("b", "a")]
    }
    ok, errs = validate_dag(parsed)
    assert not ok
    assert any("cyclic" in e.lower() or "cycle" in e.lower() for e in errs)
