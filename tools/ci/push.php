<?php

require_once ('common.php');
require_once ('vendor/autoload.php');

function buildBranch($newRef) {
  $repoUrl = Config::$REPO_URL;

  $refTarget = $newRef['target'];
  $newRefName = $newRef['name'];
  $refHead = $refTarget['hash'];
  $refMessage = $refTarget['message'];
  $refMessageLines = explode("\n", $refMessage);
  $refTitle = $refMessage;
  if (count($refMessageLines) > 1) {
    $refTitle = $refMessageLines[0];
  }
  $refAuthor = $refTarget['author']['user']['display_name'];

  echo "Processing '{$newRefName}': HEAD is '{$refHead}'\n";
  echo $refMessage . "\n";
  echo $refAuthor . "\n";

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

  $statusMessage = "{$summary}\n{$refTitle} - {$refAuthor}\n{$newRefName} @ {$refHead}.\nDetails: https://icfpc.gnoll.tech/" . $logPath;

  try {
    // Create Telegram API object
    $telegram = new Longman\TelegramBot\Telegram(Config::$TELEGRAM_API_KEY, 'skobochka_bot');

    foreach (Config::$TELEGRAM_ROOMS as $room) {
      Longman\TelegramBot\Request::sendMessage(['chat_id' => $room, 'text' => $statusMessage]);
    }

  } catch (Longman\TelegramBot\Exception\TelegramException $e) {
    // Silence is golden!
    // log telegram errors
    echo $e;
  }
}

function buildTag($newRef) {
  $repoUrl = Config::$REPO_URL;

  $newRefName = $newRef['name'];
  $refTarget = $newRef['target'];
  $refHead = $refTarget['hash'];
  $refMessage = $refTarget['message'];
  $refMessageLines = explode("\n", $refMessage);
  $refTitle = $refMessage;
  if (count($refMessageLines) > 1) {
    $refTitle = $refMessageLines[0];
  }
  $refAuthor = $refTarget['author']['user']['display_name'];

  echo "Processing '{$newRefName}': HEAD is '{$refHead}'\n";
  echo $refMessage . "\n";
  echo $refAuthor . "\n";

  $cmd = "./submission.sh '{$repoUrl}' '{$refHead}'";
  echo "Attempting to exec '{$cmd}'... \n";
  $cmdOut = []; $cmdRet = 0;
  $last = exec($cmd, $cmdOut, $cmdRet);
  /* Dirty hack */
  unset($cmdOut[0]); unset($cmdOut[1]);

  $buildResult = ($cmdRet == 0);
  $summary = "SUBMISSION " . ($buildResult ? "OK" : "FAIL");
  echo $summary . "\n";

  $statusMessage = "{$summary}\n";
  if ($buildResult) {
    foreach($cmdOut as $idx => $str) {
      if ($str == '__SUBMISSION_DATA__') {
        $submissionData = $cmdOut[$idx+1];
        $submissionData = base64_decode($submissionData);
        unset($cmdOut[$idx+1]); unset($cmdOut[$idx]);
        break;
      }
    }

    $submissionFile = 'submissions/' . $newRefName . '_' . (new DateTime())->format('Y-m-d_His') . '.tar.gz';
    file_put_contents('./' . $submissionFile, $submissionData);
    $submissionHash = md5_file($submissionFile);
    $statusMessage .= "https://icfpc.gnoll.tech/{$submissionFile} md5: {$submissionHash}\n";
  }

  $fullLog = implode("\n", $cmdOut);
  echo $fullLog; echo "\n";
  $logPath = saveBuildLog($refHead, $fullLog);
  $statusMessage .= "Log: https://icfpc.gnoll.tech/" . $logPath;

  try {
    // Create Telegram API object
    $telegram = new Longman\TelegramBot\Telegram(Config::$TELEGRAM_API_KEY, 'skobochka_bot');

    foreach (Config::$TELEGRAM_ROOMS as $room) {
      Longman\TelegramBot\Request::sendMessage(['chat_id' => $room, 'text' => $statusMessage]);
    }

  } catch (Longman\TelegramBot\Exception\TelegramException $e) {
    // Silence is golden!
    // log telegram errors
    echo $e;
  }
}

$inputJSON = file_get_contents('php://input');
$body = json_decode($inputJSON, true);

$body = $body['push'];

if (!isset($body['changes'])) {
  echo "No changes in push\n";
  die(-1);
}

$changes = $body['changes'];
echo "Callback has " . count($changes) . " changes\n";

foreach($changes as $change) {
  if (!isset($change['new'])) {
    continue;
  }

  $newRef = $change['new'];
  $newRefType = $newRef['type'];
  $newRefName = $newRef['name'];
  $refTarget = $newRef['target'];
  $refHead = $refTarget['hash'];

  $lockFile = './lock/' . $newRefName . '-' . $refHead;
  sleep(2); // Thin and shiny sleep-synchonization %)
  if (file_exists($lockFile)) {
    echo "{$newRefName} - {$refHead}: duplicate webhook detected. Skipping\n";
    continue;
  }
  touch($lockFile);

  switch ($newRefType) {
    case 'branch':
      if (!in_array($newRefName, Config::$TRACK_BRANCHES)) {
        echo "Skipping branch '{$newRefName}' bcs is not tracked\n";
        continue;
      }

      buildBranch($newRef);
      break;
    case 'tag':
      buildTag($newRef);
      break;
    default:
      echo "Skipping new ref '{$newRefName}' of type '{$newRefType}'\n";
      continue;
  }
}
