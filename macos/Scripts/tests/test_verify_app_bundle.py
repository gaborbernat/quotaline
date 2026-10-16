"""Exercise release contracts with disposable bundles and mocked Apple command boundaries."""

import base64
import plistlib
import runpy
import subprocess
import sys
from pathlib import Path
from typing import TYPE_CHECKING, Final

import pytest

from Scripts.verify_app_bundle import main

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


type Metadata = dict[str, str | bool | list[str]]


@pytest.fixture
def metadata() -> Metadata:
    """Represent a Direct build whose updater is disabled until it receives a release key."""
    return {
        "CFBundleExecutable": "Quotaline",
        "TMBDistribution": "Direct",
        "TMBSelfUpdateEnabled": "NO",
        "SUFeedURL": "https://example.invalid/feed.xml",
        "SUPublicEDKey": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "SUEnableInstallerLauncherService": True,
    }


@pytest.fixture
def app(tmp_path: Path, metadata: Metadata) -> Path:
    """Keep every bundle mutation away from installed or built applications."""
    app: Final = tmp_path / "Quotaline.app"
    (app / "Contents/MacOS").mkdir(parents=True)
    (app / "Contents/MacOS/Quotaline").touch()
    (app / "Contents/Frameworks/Sparkle.framework").mkdir(parents=True)
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    return app


@pytest.fixture
def responses(mocker: MockerFixture) -> dict[str, str]:
    """Return tool output by command; validation still reads the real bundle files."""
    values: Final = {
        "file": "Mach-O 64-bit executable",
        "lipo": "arm64 x86_64",
        "otool": "Sparkle.framework",
        "dyld_info": "Sparkle.framework",
        "signature": "TeamIdentifier=not set\n",
        "app-entitlements": "",
        "widget-entitlements": "",
    }

    def respond(arguments: tuple[str, ...], **_options: bool | int) -> subprocess.CompletedProcess[str]:
        tool: Final = Path(arguments[0]).name
        if tool == "codesign":
            key: Final = (
                ("widget-entitlements" if arguments[-1].endswith(".appex") else "app-entitlements")
                if "--entitlements" in arguments
                else "signature"
            )
            output = values[key]
        elif tool == "file" and not arguments[-1].endswith("/Quotaline"):
            output = "XML document"
        else:
            output = values[arguments[1] if tool == "xcrun" else tool]
        return subprocess.CompletedProcess(arguments, 0, stdout=output, stderr="")

    mocker.patch("subprocess.run", autospec=True, side_effect=respond)
    return values


@pytest.mark.usefixtures("responses")
def test_direct_bundle_accepts_placeholder_when_updates_disabled(app: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Unsigned CI builds validate structure without a release signing key."""
    main([str(app), "Direct", "required"])
    assert "signing=ad-hoc" in capsys.readouterr().out


@pytest.mark.usefixtures("responses")
@pytest.mark.parametrize(
    ("field", "value", "message"),
    [
        ("TMBDistribution", "Homebrew", "expected Direct"),
        ("SUPublicEDKey", "", "empty or unexpanded"),
        ("SUPublicEDKey", "$(UNSET)", "empty or unexpanded"),
    ],
)
def test_metadata_contract_fails(app: Path, metadata: Metadata, field: str, value: str, message: str) -> None:
    """Incorrect metadata cannot reach a published artifact."""
    metadata[field] = value
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    with pytest.raises(SystemExit, match=message):
        main([str(app), "Direct", "required"])


@pytest.mark.usefixtures("responses")
def test_unknown_updater_policy_fails(app: Path) -> None:
    """A misspelled policy cannot silently turn off updater checks."""
    with pytest.raises(SystemExit, match="required or forbidden"):
        main([str(app), "Direct", "optional"])


@pytest.mark.parametrize(
    ("tool", "value", "message"),
    [
        ("file", "XML document", "no Mach-O"),
        ("lipo", "arm64", "not universal"),
        ("otool", "", "no Sparkle dependency"),
        ("dyld_info", "", "no Sparkle dependency"),
    ],
)
def test_binary_contract_fails(app: Path, responses: dict[str, str], tool: str, value: str, message: str) -> None:
    """The bundle must be universal and agree with both dependency inspectors."""
    responses[tool] = value
    with pytest.raises(SystemExit, match=message):
        main([str(app), "Direct", "required"])


@pytest.mark.usefixtures("responses")
@pytest.mark.parametrize("missing", ["framework", "metadata"])
def test_direct_requires_updater(app: Path, metadata: Metadata, missing: str) -> None:
    """An updater-enabled distribution needs both code and metadata."""
    if missing == "framework":
        (app / "Contents/Frameworks/Sparkle.framework").rmdir()
    else:
        del metadata["SUFeedURL"]
        (app / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    with pytest.raises(SystemExit, match=r"does not embed|missing Sparkle"):
        main([str(app), "Direct", "required"])


@pytest.mark.usefixtures("responses")
@pytest.mark.parametrize(
    ("key", "valid"),
    [
        (base64.b64encode(bytes(range(32))).decode(), True),
        ("invalid!", False),
        ("YQ==", False),
        ("A" * 43 + "=", False),
    ],
    ids=["release-key", "invalid-base64", "short-key", "placeholder"],
)
def test_enabled_updater_requires_real_key(
    app: Path, metadata: Metadata, key: str, capsys: pytest.CaptureFixture[str], *, valid: bool
) -> None:
    """Self-update cannot ship with an invalid key or the unsigned-build placeholder."""
    metadata.update(TMBSelfUpdateEnabled="YES", SUPublicEDKey=key)
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    if valid:
        main([str(app), "Direct", "required"])
        assert "verified Direct app" in capsys.readouterr().out
    else:
        with pytest.raises(SystemExit, match="EdDSA public key"):
            main([str(app), "Direct", "required"])


@pytest.fixture
def homebrew(app: Path, metadata: Metadata, responses: dict[str, str]) -> Path:
    """Remove updater artifacts as the Homebrew and App Store configurations do."""
    for name in ("SUFeedURL", "SUPublicEDKey", "SUEnableInstallerLauncherService"):
        del metadata[name]
    metadata["TMBDistribution"] = "Homebrew"
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    (app / "Contents/Frameworks/Sparkle.framework").rmdir()
    responses.update(otool="", dyld_info="")
    return app


def test_homebrew_has_no_updater(homebrew: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """A complete updater-free bundle passes its own distribution contract."""
    main([str(homebrew), "Homebrew", "forbidden"])
    assert "verified Homebrew app" in capsys.readouterr().out


@pytest.mark.parametrize("violation", ["enabled", "framework", "dependency", "metadata"])
def test_homebrew_rejects_updater(
    homebrew: Path, metadata: Metadata, responses: dict[str, str], violation: str
) -> None:
    """Each residual updater component independently prevents release."""
    if violation == "enabled":
        metadata["TMBSelfUpdateEnabled"] = "YES"
    elif violation == "framework":
        (homebrew / "Contents/Frameworks/Sparkle.framework").mkdir()
    elif violation == "dependency":
        responses["otool"] = "Sparkle.framework"
    else:
        metadata["SUFeedURL"] = "https://example.invalid/feed.xml"
    (homebrew / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    with pytest.raises(SystemExit, match=r"self-update|Sparkle"):
        main([str(homebrew), "Homebrew", "forbidden"])


@pytest.fixture
def signed(app: Path, metadata: Metadata, responses: dict[str, str]) -> Path:
    """Use matching app and widget entitlements for the signed Direct distribution."""
    group: Final = "TEAM.tech.bernat.quotaline"
    metadata["QuotalineAppGroup"] = group
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    widget: Final = app / "Contents/PlugIns/QuotalineWidget.appex/Contents"
    widget.mkdir(parents=True)
    (widget / "Info.plist").write_bytes(plistlib.dumps({"QuotalineAppGroup": group}))
    responses["signature"] = "TeamIdentifier=TEAM\nflags=0x10000(runtime)\n"
    for name, sandbox in (("app", False), ("widget", True)):
        responses[f"{name}-entitlements"] = plistlib.dumps(
            {"com.apple.security.application-groups": [group], "com.apple.security.app-sandbox": sandbox}
        ).decode()
    return app


def test_signed_direct_bundle(signed: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Distribution signatures require hardened runtime and matching widget entitlements."""
    main([str(signed), "Direct", "required"])
    assert "signing=distribution" in capsys.readouterr().out


@pytest.mark.parametrize("violation", ["runtime", "widget", "group", "metadata", "sandbox"])
def test_signed_bundle_contract_fails(
    signed: Path, metadata: Metadata, responses: dict[str, str], violation: str
) -> None:
    """Signing cannot hide a missing widget or mismatched sandbox identity."""
    if violation == "runtime":
        responses["signature"] = "TeamIdentifier=TEAM\n"
    elif violation == "widget":
        widget: Final = signed / "Contents/PlugIns/QuotalineWidget.appex"
        (widget / "Contents/Info.plist").unlink()
        (widget / "Contents").rmdir()
        widget.rmdir()
    elif violation == "group":
        responses["app-entitlements"] = plistlib.dumps({"com.apple.security.application-groups": []}).decode()
    elif violation == "metadata":
        metadata["QuotalineAppGroup"] = "wrong"
        (signed / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    else:
        responses["widget-entitlements"] = plistlib.dumps(
            {"com.apple.security.application-groups": ["TEAM.tech.bernat.quotaline"]}
        ).decode()
    with pytest.raises(SystemExit, match=r"runtime|missing signed bundle|app group|sandbox"):
        main([str(signed), "Direct", "required"])


def test_app_store_signature(
    signed: Path, metadata: Metadata, responses: dict[str, str], capsys: pytest.CaptureFixture[str]
) -> None:
    """Store builds use the registered group and sandbox both bundles."""
    group: Final = "group.tech.bernat.quotaline"
    metadata.update(TMBDistribution="App Store", QuotalineAppGroup=group)
    for name in ("SUFeedURL", "SUPublicEDKey", "SUEnableInstallerLauncherService"):
        del metadata[name]
    (signed / "Contents/Frameworks/Sparkle.framework").rmdir()
    (signed / "Contents/Info.plist").write_bytes(plistlib.dumps(metadata))
    (signed / "Contents/PlugIns/QuotalineWidget.appex/Contents/Info.plist").write_bytes(
        plistlib.dumps({"QuotalineAppGroup": group})
    )
    responses.update(otool="", dyld_info="", signature="TeamIdentifier=TEAM\n")
    responses["app-entitlements"] = responses["widget-entitlements"] = plistlib.dumps(
        {"com.apple.security.application-groups": [group], "com.apple.security.app-sandbox": True}
    ).decode()
    main([str(signed), "App Store", "forbidden"])
    assert "verified App Store app" in capsys.readouterr().out


def test_cli_ad_hoc_signature(
    app: Path, responses: dict[str, str], mocker: MockerFixture, capsys: pytest.CaptureFixture[str]
) -> None:
    """An unsigned codesign response follows the same contract through the CLI."""
    responses["signature"] = ""
    mocker.patch.object(sys, "argv", ["verify_app_bundle.py", str(app), "Direct", "required"])
    runpy.run_path(str(Path(__file__).resolve().parents[1] / "verify_app_bundle.py"), run_name="__main__")
    assert "signing=ad-hoc" in capsys.readouterr().out
