from src.config import settings

def test_llm_timeout_config():
    """Verify that LLM_TIMEOUT is present and has the default value."""
    assert hasattr(settings, "LLM_TIMEOUT")
    assert settings.LLM_TIMEOUT == 30
