from setuptools import setup, find_packages

setup(
    name="jorm",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "typer>=0.9.0",
        "argcomplete"
    ],
    entry_points={
        "console_scripts": [
            "jorm = jorm.cli:app"
        ]
    },
    author="Sebx",
    description="CLI para ejecución y validación de DAGs en texto plano o Markdown.",
    include_package_data=True,
)
