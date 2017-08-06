<?php

require_once __DIR__ . '/vendor/autoload.php';

function srvfetchStatus() {
  $curl = new \Curl\Curl();
  $curl->setUserAgent('SkobochkaBenchmarkBot/1.0.0 (+http://icfpc.gnoll.tech/bot.html)');
  $curl->setOpt(CURLOPT_FOLLOWLOCATION, true);

  $curl->get('http://punter.inf.ed.ac.uk/status.html');
  if ($curl->error) {
    throw new RuntimeException($curl->errorMessage, $curl->errorCode);
  }

  $doc = DOMDocument::loadHTML($curl->response);

  if ($doc === false) {
    throw new RuntimeException('Unable to parse input');
  }

  $xpath = new DOMXpath($doc);

  $match = [];

  $elements = $xpath->query("//h3");
  $genInfoRaw = $elements->item(0)->nodeValue;
  if (preg_match('/Information generated: (.+)$/ui', $genInfoRaw, $matched)) {
    $ts = $matched[1] . '+02:00';
    $ts = new DateTime($ts);
    $ts->setTimezone(new DateTimeZone('UTC'));
  }

  $elements = $xpath->query("//tr[position()>1]");

  $data = [];

  if (!is_null($elements)) {
    foreach ($elements as $element) {

      $nodes = $element->childNodes;

      $nowPunters = -1; $maxPunters = -1;
      $srvStatusRaw = $nodes->item(0)->nodeValue;
      $srvStatus = 'UNKNOWN';
      if ($srvStatusRaw == 'Offline.') {
        $srvStatus = 'OFFLINE';
      }
      elseif ($srvStatusRaw == 'Game in progress.') {
        $srvStatus = 'IN_PROGRESS';
      }
      elseif (preg_match('/Waiting for punters\. \(([0-9]+)\/([0-9]+)\)/ui', $srvStatusRaw, $match) > 0) {
        $srvStatus = 'WAIT';
        $nowPunters = (int)($match[1]); $maxPunters = (int)($match[2]);
      }
      $puntersRaw = $nodes->item(1)->nodeValue;
      $punters = array_filter(array_map('trim', explode(',', $puntersRaw)), 'strlen');

      $extensionsRaw = $nodes->item(2)->nodeValue;
      $extensions = array_filter(array_map('trim', explode(',', $extensionsRaw)), 'strlen');

      $data[] = [
        'status' => $srvStatus,
        'punters' => [
          'names' => $punters,
          'count' => $nowPunters,
          'maxCount' => $maxPunters,
        ],
        'extensions' => $extensions,
        'port' => (int)($nodes->item(4)->nodeValue),
        'map' => [
          'name' => $nodes->item(5)->nodeValue,
          'shortName' => substr($nodes->item(5)->nodeValue, 0, strpos($nodes->item(5)->nodeValue, '.json')),
          'url' => 'http://punter.inf.ed.ac.uk/maps/' . $nodes->item(5)->nodeValue,
        ],
      ];
    }
  }

  return ['result' => true, 'ts' => $ts, 'servers' => $data];
}

function srvPrepareList($srvList) {
  $outList = [];
  foreach($srvList as $srv) {
    if ($srv['status'] != 'WAIT') {
      continue;
    }

    /* Fucking buggy stats %( */
    if ($srv['punters']['maxCount'] == $srv['punters']['count']) {
      continue;
    }

    $outList[] = $srv;
  }
  usort($outList, function ($l, $r) {
    $lSlots = $l['punters']['maxCount'] - $l['punters']['count'];
    $rSlots = $r['punters']['maxCount'] - $r['punters']['count'];
    return $lSlots - $rSlots;
  });
  return $outList;
}

function srvAllocateSlot(&$srvList, $map) {
  foreach($srvList as $key => $srv) {
    if ($srv['map']['shortName'] != $map) {
      continue;
    }

    /* Don't play with yourself */
    if (in_array('skobochka', $srv['punters']['names'])) {
      continue;
    }

    unset($srvList[$key]);
    return $srv;
  }

  return false;
}