> **Superseded 2026-11-13 by [`install-guide.md`](install-guide.md)** (LexPH, Phase 8). This page
> describes the Kristoff builds 0.1.x, which had a built-in key.

# Installing Kristoff on your Mac

Kristoff is a Mac app for Apple Silicon Macs (M1 or newer) running macOS 14 or later. It keeps your
documents and chats on your Mac. The only outside connection it makes is to the Claude API, to
answer your questions.

## 1. Get the app

1. You will receive one file, `Kristoff-<version>-arm64.dmg`, by AirDrop or on a USB stick.
2. Double-click the `.dmg`. A window opens with the Kristoff icon.
3. Drag **Kristoff** onto the **Applications** folder in that window.
4. Eject the disk image (the eject button next to "Kristoff" in Finder's sidebar). You can then
   delete the `.dmg` file.

## 2. Open it the first time

Kristoff is not from the App Store, so macOS blocks the first open. You only need to do this once.

1. Open **Applications** and double-click **Kristoff**.
2. macOS shows "Kristoff" Not Opened. Click **Done**. (Do not click Move to Trash.)
3. Open **System Settings → Privacy & Security**. Scroll down to the Security section.
4. Next to the message that Kristoff was blocked, click **Open Anyway**. Confirm with your password
   or Touch ID, then click **Open Anyway** again if asked.
5. Kristoff opens straight to its Home screen. From now on it opens normally. There is no code
   or password to enter.

If **Open Anyway** does not appear, you can do the same in Terminal instead. Paste this line and
press Return:

```
xattr -dr com.apple.quarantine /Applications/Kristoff.app
```

Then double-click Kristoff again.

## 3. Your documents and chats

- Everything you upload and every chat is stored only on this Mac, in:
  `~/Library/Application Support/app.kristoff.desktop/`
  (In Finder: **Go → Go to Folder…**, paste that path, press Return.)
- **Back up:** quit Kristoff, then copy that whole folder to a USB drive or a cloud folder. To
  restore, quit Kristoff and copy the folder back to the same place.
- Deleting Kristoff from Applications does not delete that folder.

## 4. Things to know

- **"File deleted"** on a source card means the answer cited a PDF that you have since deleted
  from your Library. The quoted passage stays in the chat, but the PDF can no longer be opened.
  Upload it again if you need it.
- **Statute search covers Republic Acts and the 1987 Constitution only.** It does not include the
  Revised Penal Code, the Family Code, the Labor Code or the Rules of Court. To use one of those,
  upload it as a PDF and attach it to your chat.
- Answers can still be wrong. Check important citations against the official source (SC E-Library,
  Official Gazette, Lawphil) before you rely on them.

## Owner test checklist (run on the final DMG before handing it over)

Run these on a second macOS user account, in order. Each must pass.

1. Install the DMG on the second account (sections 1 and 2 above, including Open Anyway).
2. Kristoff opens straight to Home, with no code screen.
3. Upload a PDF; the progress bar finishes and the file shows as ready in the Library.
4. Ask a mixed question that needs both the uploaded PDF and a statute.
5. Open both kinds of citation: a PDF citation opens the file at the cited page, and a statute
   citation opens its official page in the browser.
6. Dictate a question with the microphone button.
7. Delete the file; it disappears from the Library and from the chat's attachments.
8. Quit and reopen Kristoff; it opens straight to Home and your chats and files are still there.
