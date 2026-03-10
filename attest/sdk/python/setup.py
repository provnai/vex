"""Setup configuration for attest-agent package."""

import os
from setuptools import setup, find_packages

this_dir = os.path.abspath(os.path.dirname(__file__))

with open(os.path.join(this_dir, "README.md"), "r", encoding="utf-8") as f:
    long_description = f.read()

with open(os.path.join(this_dir, "requirements.txt"), "r", encoding="utf-8") as f:
    requirements = [
        line.strip() for line in f if line.strip() and not line.startswith("#")
    ]

setup(
    name="attest-agent",
    version="0.1.0",
    description="Python SDK for the Attest verifiable agent action system",
    long_description=long_description,
    long_description_content_type="text/markdown",
    author="Attest Contributors",
    author_email="dev@attest.dev",
    url="https://github.com/anomalyco/attest",
    license="MIT",
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Security :: Cryptography",
        "Topic :: Software Development :: Libraries :: Python Modules",
    ],
    packages=find_packages(exclude=["tests", "tests.*"]),
    python_requires=">=3.7",
    install_requires=requirements,
    extras_require={
        "langchain": ["langchain>=0.1.0"],
        "dev": ["pytest>=7.0.0", "pytest-cov>=4.0.0", "black>=23.0.0", "mypy>=1.0.0"],
    },
    entry_points={
        "console_scripts": [
            "attest-agent=attest_client:main_cli",
        ],
    },
    keywords=[
        "attestation",
        "cryptography",
        "agent",
        "security",
        "verification",
        "langchain",
        "ai-agents",
    ],
    project_urls={
        "Bug Reports": "https://github.com/anomalyco/attest/issues",
        "Source": "https://github.com/anomalyco/attest",
        "Documentation": "https://attest.readthedocs.io/",
    },
)
