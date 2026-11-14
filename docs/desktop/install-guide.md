# Installing LexPH on your Mac

LexPH is a Philippine legal research assistant for Apple Silicon Macs (M1 or newer) running macOS 14
or later. It searches Philippine laws and Supreme Court decisions on your Mac, reads the PDFs you
upload, and answers with citations. It uses Claude, so you need your own Claude API key (step 3).

## 1. Get the app

1. You will receive one file, `LexPH-<version>-arm64.dmg` (about 1.4 GB), by AirDrop or a shared
   link.
2. Double-click the `.dmg`. A window opens with the LexPH icon.
3. Drag **LexPH** onto the **Applications** folder in that window.
4. Eject the disk image (the eject button next to "LexPH" in Finder's sidebar). You can then delete
   the `.dmg` file.

## 2. Open it the first time

LexPH is not from the App Store and is not notarised by Apple, so macOS blocks the first open. You
only need to do this once.

1. Open **Applications** and double-click **LexPH**.
2. macOS shows "LexPH" Not Opened. Click **Done**. (Do not click Move to Trash.)
3. Open **System Settings → Privacy & Security**. Scroll down to the Security section.
4. Next to the message that LexPH was blocked, click **Open Anyway**. Confirm with your password or
   Touch ID, then click **Open Anyway** again if asked.

If **Open Anyway** does not appear, paste this line in Terminal and press Return, then open LexPH
again:

```
xattr -dr com.apple.quarantine /Applications/LexPH.app
```

## 3. Connect your Claude account

On first launch LexPH loads the law index, then opens **Connect your Claude account**.

1. Click **Anthropic console**, sign in or create an account, and add a payment method or credit.
2. Under **API keys**, create a key and copy it. It starts with `sk-ant-`.
3. Paste it in LexPH and click **Save**. LexPH checks the key with Anthropic before saving it.

- The key is kept in your Mac's Keychain. LexPH sends it only to Anthropic.
- You pay Anthropic directly for your own questions. **Settings → Daily question limit** (100 by
  default) is a safety brake on that bill. You can also set a monthly spend limit in the Anthropic
  console.
- To change or remove the key later, open **Settings** (bottom of the sidebar).

## 4. Your documents and chats

- Everything you upload and every chat is stored only on this Mac, in
  `~/Library/Application Support/ph.lexph.desktop/`
  (Finder: **Go → Go to Folder…**, paste that path, press Return).
- **Back up:** quit LexPH, then copy that whole folder somewhere safe. To restore, quit LexPH and
  copy it back.
- Deleting LexPH from Applications does not delete that folder or the saved key.

## 5. Things to know

- **Your questions and attached PDFs are sent to Anthropic** under your own API account, to answer
  them. Nothing else leaves your Mac.
- **LexPH is a research aid, not legal advice.** Answers can be wrong. Check every important
  citation against the official source (SC E-Library, Official Gazette, Lawphil) before you rely on
  it.
- **Coverage:** the bundled laws and decisions come from public datasets and may be incomplete or
  not the latest amended text. Laws are shown as enacted.
- **Decisions** show party names exactly as the Supreme Court published them.
- **Data credit:** statute and decision data from
  [OmniCorpus](https://huggingface.co/datasets/mongramosjr/philippine-omnicorpus) (ODC-By), with
  credit to its author. Philippine government works carry no copyright (RA 8293, Sec. 176).

## Owner test checklist (run on the final DMG before sharing it)

Run these on a fresh macOS user account, in order. Each must pass.

1. Install the DMG (sections 1 and 2, including Open Anyway).
2. After the loading screen, **Connect your Claude account** opens by itself.
3. Paste a wrong key: Anthropic rejects it and nothing is saved.
4. Paste a working key: "Key saved", and Send turns on.
5. Ask a question that needs a statute; the answer cites the law.
6. Upload a PDF, ask about it, and open the PDF citation at its page.
7. Settings → Remove the key: Send turns off and shows "Add your Claude API key in Settings".
8. Quit and reopen; chats and files are still there.
