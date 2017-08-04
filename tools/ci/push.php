<?php

require_once ('common.php');

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
  saveBuildLog($refHead, $fullLog);
  $buildResult = ($cmdRet == 0);
  echo "BUILD " . ($buildResult ? "OK" : "FAIL") . "\n";
  // TODO: Broadcast to Telegram
}
