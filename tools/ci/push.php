<?php

require_once ('common.php');
require_once ('vendor/autoload.php');

$repoUrl = Config::$REPO_URL;

$inputJSON = file_get_contents('php://input');
$body = json_decode($inputJSON, true);

$body = $body['push'];

if (!isset($body['changes'])) {
  echo "No changes in push\n";
  die(-1);
}

$changes = $body['changes'];
foreach($changes as $change) {
  if (!isset($change['new'])) {
    continue;
  }

  $newRef = $change['new'];
  $newRefType = $newRef['type'];
  $newRefName = $newRef['name'];

  if ($newRefType !== 'branch') {
    echo "Skipping new ref '{$newRefName}' of type '{$newRefType}'\n";
    continue;
  }

  if (!in_array($newRefName, Config::$TRACK_BRANCHES)) {
    echo "Skipping branch '{$newRefName}' bcs is not tracked\n";
    continue;
  }

  $refTarget = $newRef['target'];
  $refHead = $refTarget['hash'];

  echo "Processing '{$newRefName}': HEAD is '{$refHead}'\n";

  $cmd = "./build.sh '{$repoUrl}' '{$refHead}'";
  echo "Attempting to exec '{$cmd}'... \n";
  $cmdOut = []; $cmdRet = 0;
  $last = exec($cmd, $cmdOut, $cmdRet);
  /* Dirty hack */
  unset($cmdOut[0]); unset($cmdOut[1]);

  $fullLog = implode("\n", $cmdOut);
  echo $fullLog; echo "\n";
  $logPath = saveBuildLog($refHead, $fullLog);
  $buildResult = ($cmdRet == 0);
  $summary = "BUILD " . ($buildResult ? "OK" : "FAIL");
  echo $summary . "\n";

  $statusMessage = "{$newRefName} @ {$refHead}: {$summary}. Details: https://icfpc.gnoll.tech/" . $logPath;

  try {
    // Create Telegram API object
    $telegram = new Longman\TelegramBot\Telegram(Config::$TELEGRAM_API_KEY, 'skobochka_bot');

    foreach (Config::$TELEGRAM_ROOMS as $room) {
      Longman\TelegramBot\Request::sendMessage(['chat_id' => $room, 'text' => $statusMessage]);
    }

  } catch (Longman\TelegramBot\Exception\TelegramException $e) {
    // Silence is golden!
    // log telegram errors
    // echo $e;
  }
}
