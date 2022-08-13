#!/bin/bash
FULLSCORE="62"
TESTSCORE=`grep -o success output | wc -l`
if [ "$FULLSCORE" = "$TESTSCORE" ];then
   echo "ALL tests pass!"
else
   echo "Some tests failed! ("$TESTSCORE"/"$FULLSCORE")"
fi