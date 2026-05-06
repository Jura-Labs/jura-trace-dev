# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Minimal CLIP BPE tokeniser — standalone numpy-only implementation.

JTV-143 (2026-05-03): vendored from open_clip 3.3.0 with torch dependencies
replaced by numpy. Returns numpy int32 arrays of shape (batch, context_length)
suitable for direct ONNX runtime consumption. The sidecar would otherwise need
to import torch just to tokenise five fixed prompts, which defeats the
~700 MB savings of switching to ONNX.

The vocabulary file `bpe_simple_vocab_16e6.txt.gz` ships alongside this module
in `sidecar/app/services/` and is bundled by PyInstaller via collect_data_files.

Original copyright: MIT licence, OpenAI / OpenCLIP contributors:
Gabriel Ilharco, Mitchell Wortsman, Nicholas Carlini, Rohan Taori, Achal Dave,
Vaishaal Shankar, John Miller, Hongseok Namkoong, Hannaneh Hajishirzi,
Ali Farhadi, Ludwig Schmidt.
"""

from __future__ import annotations

import gzip
import html
import os
from functools import lru_cache
from typing import List, Union

import numpy as np
import regex as re


__all__ = ["SimpleTokenizer", "tokenize"]

DEFAULT_CONTEXT_LENGTH = 77
SPECIAL_TOKENS = ["<start_of_text>", "<end_of_text>"]


def _default_bpe_path() -> str:
    return os.path.join(os.path.dirname(__file__), "bpe_simple_vocab_16e6.txt.gz")


@lru_cache()
def bytes_to_unicode():
    """Reversible mapping between utf-8 byte values and unicode strings.

    The CLIP BPE vocabulary was produced over this representation; the
    reversible escape avoids whitespace / control characters in BPE merges.
    """
    bs = (
        list(range(ord("!"), ord("~") + 1))
        + list(range(ord("¡"), ord("¬") + 1))
        + list(range(ord("®"), ord("ÿ") + 1))
    )
    cs = bs[:]
    n = 0
    for b in range(2 ** 8):
        if b not in bs:
            bs.append(b)
            cs.append(2 ** 8 + n)
            n += 1
    cs = [chr(n) for n in cs]
    return dict(zip(bs, cs))


def get_pairs(word):
    """Return the set of symbol pairs in a word (tuple of strings)."""
    pairs = set()
    prev_char = word[0]
    for char in word[1:]:
        pairs.add((prev_char, char))
        prev_char = char
    return pairs


def basic_clean(text: str) -> str:
    text = html.unescape(html.unescape(text))
    return text.strip()


def whitespace_clean(text: str) -> str:
    return " ".join(text.split())


class SimpleTokenizer:
    """Byte-Pair Encoding tokeniser used by all OpenAI / OpenCLIP CLIP models.

    Output format is a numpy int32 array of shape (batch, context_length).
    Each row begins with the start-of-text token, ends with the end-of-text
    token, and is right-padded with zeros to context_length=77 (or whatever
    is supplied at call time).
    """

    def __init__(self, bpe_path: str | None = None, context_length: int = DEFAULT_CONTEXT_LENGTH):
        if bpe_path is None:
            bpe_path = _default_bpe_path()
        self.byte_encoder = bytes_to_unicode()
        self.byte_decoder = {v: k for k, v in self.byte_encoder.items()}
        merges = gzip.open(bpe_path).read().decode("utf-8").split("\n")
        merges = merges[1 : 49152 - 256 - 2 + 1]
        merges = [tuple(merge.split()) for merge in merges]
        vocab = list(bytes_to_unicode().values())
        vocab = vocab + [v + "</w>" for v in vocab]
        for merge in merges:
            vocab.append("".join(merge))
        vocab.extend(SPECIAL_TOKENS)
        self.encoder = {v: i for i, v in enumerate(vocab)}
        self.decoder = {v: k for k, v in self.encoder.items()}
        self.bpe_ranks = dict(zip(merges, range(len(merges))))
        self.cache = {tok: tok for tok in SPECIAL_TOKENS}
        self.pat = re.compile(
            r"""<start_of_text>|<end_of_text>|'s|'t|'re|'ve|'m|'ll|'d|"""
            r"""[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+""",
            re.IGNORECASE,
        )
        self.context_length = context_length
        self.sot_token_id = self.encoder["<start_of_text>"]
        self.eot_token_id = self.encoder["<end_of_text>"]

    def bpe(self, token: str) -> str:
        if token in self.cache:
            return self.cache[token]
        word = tuple(token[:-1]) + (token[-1] + "</w>",)
        pairs = get_pairs(word)
        if not pairs:
            return token + "</w>"
        while True:
            bigram = min(pairs, key=lambda pair: self.bpe_ranks.get(pair, float("inf")))
            if bigram not in self.bpe_ranks:
                break
            first, second = bigram
            new_word = []
            i = 0
            while i < len(word):
                try:
                    j = word.index(first, i)
                    new_word.extend(word[i:j])
                    i = j
                except ValueError:
                    new_word.extend(word[i:])
                    break
                if word[i] == first and i < len(word) - 1 and word[i + 1] == second:
                    new_word.append(first + second)
                    i += 2
                else:
                    new_word.append(word[i])
                    i += 1
            new_word = tuple(new_word)
            word = new_word
            if len(word) == 1:
                break
            pairs = get_pairs(word)
        word = " ".join(word)
        self.cache[token] = word
        return word

    def encode(self, text: str) -> List[int]:
        bpe_tokens: List[int] = []
        text = whitespace_clean(basic_clean(text)).lower()
        for token in re.findall(self.pat, text):
            token = "".join(self.byte_encoder[b] for b in token.encode("utf-8"))
            bpe_tokens.extend(self.encoder[bpe_token] for bpe_token in self.bpe(token).split(" "))
        return bpe_tokens

    def __call__(self, texts: Union[str, List[str]], context_length: int | None = None) -> np.ndarray:
        """Tokenise text(s) and return a numpy int32 array padded to context_length.

        Mirrors the open_clip SimpleTokenizer.__call__ contract but emits numpy
        instead of torch tensors so callers can feed the array directly into
        an onnxruntime InferenceSession.
        """
        if isinstance(texts, str):
            texts = [texts]
        ctx = context_length or self.context_length

        # int64 to match the open_clip reference and the dtype expected by
        # the ONNX-exported text encoder graph (Long input tensor).
        result = np.zeros((len(texts), ctx), dtype=np.int64)
        for i, text in enumerate(texts):
            tokens = [self.sot_token_id] + self.encode(text) + [self.eot_token_id]
            if len(tokens) > ctx:
                tokens = tokens[:ctx]
                tokens[-1] = self.eot_token_id
            result[i, : len(tokens)] = tokens
        return result


_singleton: SimpleTokenizer | None = None


def get_tokenizer() -> SimpleTokenizer:
    """Return a process-wide cached SimpleTokenizer instance."""
    global _singleton
    if _singleton is None:
        _singleton = SimpleTokenizer()
    return _singleton


def tokenize(texts: Union[str, List[str]], context_length: int = DEFAULT_CONTEXT_LENGTH) -> np.ndarray:
    """Convenience helper — tokenise via the cached singleton."""
    return get_tokenizer()(texts, context_length=context_length)
