try:
    from .avap_isa import AvapISA, __version__
except ImportError as e:
    raise ImportError(f"avap-isa native extension not found. Run 'maturin develop'.\n{e}") from e

__all__ = ["AvapISA", "__version__"]
